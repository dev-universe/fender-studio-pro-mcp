//! Fresh, bounded digests of the exposed parameter projection, not plug-in state.
//!
//! Format fsp-parameter-digest-v1 uses SHA256 and u64 little-endian byte-length
//! frames. Each stream starts with framed UTF-8 format, stream name, source,
//! and framed u32 little-endian count. Records are framed UTF-8 JSON with object
//! keys recursively sorted, no whitespace, and native floats represented only
//! by uppercase IEEE bits. Schema records include all indices. Value records
//! include index/raw/normalized/display. The final all_values/non_read_only
//! hashes use their own framed header followed by framed schema-stream SHA256
//! bytes and framed selected value-stream SHA256 bytes. Thus every schema fact
//! binds both value digests without retaining the complete snapshot in memory.
//!
//! Only read_only=true AND effective writable=false observations are omitted
//! from non_read_only. Disabled, unsupported and opaque non-read-only records
//! remain represented. An opaque value's contents are never verified. Failure
//! at any scan, native-owner release or publication boundary exposes no digest.
use super::*;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

const FORMAT: &str = "fsp-parameter-digest-v1";
const MAX_PARAMETERS: u32 = 8192;
const CHUNK: u32 = 128;

#[derive(Clone)]
struct Observation {
    index: u32,
    dsp: Option<fender_native_dsp_api::Parameter>,
    parameter: parameters::Description,
}

trait Reader {
    fn count(&mut self) -> Result<u32, String>;
    fn chunk(
        &mut self,
        offset: u32,
        limit: u32,
        expected_count: u32,
        guard: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<Vec<Observation>, String>;
}

struct NativeReader<'a, 'b> {
    ctx: &'a Context<'b>,
    provider: &'a Provider,
}
impl Reader for NativeReader<'_, '_> {
    fn count(&mut self) -> Result<u32, String> {
        match self.provider {
            Provider::Dsp(provider) => provider.metadata.count().map_err(|e| e.to_string()),
            Provider::Controller(controller) => controller.count(),
            _ => Err("parameter digest requires an indexed provider".into()),
        }
    }
    fn chunk(
        &mut self,
        offset: u32,
        limit: u32,
        expected_count: u32,
        guard: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<Vec<Observation>, String> {
        match self.provider {
            Provider::Dsp(provider) => {
                guard()?;
                let page = provider.metadata.page(offset, limit);
                guard()?;
                let page = page.map_err(|e| e.to_string())?;
                if page.reported_count != expected_count
                    || page.offset != offset
                    || !page.failed_indices.is_empty()
                    || page.records.len() != page.attempted as usize
                {
                    return Err("incomplete DSP metadata page".into());
                }
                let controller = provider.direct.as_ref().map_err(Clone::clone)?;
                let mut observations = Vec::with_capacity(page.records.len());
                for record in page.records {
                    let parameter = observe(
                        || controller.find(&record.name, self.ctx.api),
                        |parameter| parameter.describe().map_err(|e| e.to_string()),
                        guard,
                    )?;
                    observations.push(Observation {
                        index: record.index,
                        dsp: Some(record),
                        parameter,
                    });
                }
                Ok(observations)
            }
            Provider::Controller(controller) => {
                let mut observations = Vec::with_capacity(limit as usize);
                for index in offset..offset + limit {
                    let parameter = observe(
                        || controller.at(index, self.ctx.api),
                        |parameter| parameter.describe().map_err(|e| e.to_string()),
                        guard,
                    )?;
                    observations.push(Observation {
                        index,
                        dsp: None,
                        parameter,
                    });
                }
                Ok(observations)
            }
            _ => Err("parameter digest requires an indexed provider".into()),
        }
    }
}

fn observe<T, F, R>(
    lookup: F,
    read: R,
    guard: &mut dyn FnMut() -> Result<(), String>,
) -> Result<parameters::Description, String>
where
    F: FnOnce() -> Result<T, String>,
    R: FnOnce(&T) -> Result<parameters::Description, String>,
{
    guard()?;
    // Lookup pins borrowed native objects before a callback can invalidate them.
    let parameter = lookup();
    let result = match (guard(), parameter.as_ref()) {
        (Ok(()), Ok(parameter)) => read(parameter),
        (Err(reason), _) => Err(reason),
        (_, Err(reason)) => Err(reason.clone()),
    };
    let before_release = guard();
    drop(parameter);
    let after_release = guard();
    before_release?;
    after_release?;
    result
}

pub(super) fn inspect(ctx: &Context<'_>, provider: &Provider) -> Result<Value, String> {
    let mut reader = NativeReader { ctx, provider };
    let mut guard = || {
        provider.check()?;
        ctx.check()
    };
    let value = scan(provider.source(), &mut reader, &mut guard)?;
    guard()?;
    Ok(value)
}

pub(super) fn invalidate(value: &mut Value) {
    if value.get("hash_format_version").is_some() {
        let value = value.as_object_mut().expect("digest reply object");
        for key in [
            "schema_sha256",
            "all_values_sha256",
            "non_read_only_values_sha256",
        ] {
            value.remove(key);
        }
        value.insert("access_complete".into(), json!(false));
    }
}

fn frame(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}
fn stream(name: &str, source: &str, count: u32) -> Sha256 {
    let mut hash = Sha256::new();
    for value in [
        FORMAT.as_bytes(),
        name.as_bytes(),
        source.as_bytes(),
        &count.to_le_bytes(),
    ] {
        frame(&mut hash, value);
    }
    hash
}
fn canonical_json(value: &Value) -> Vec<u8> {
    fn write(value: &Value, out: &mut Vec<u8>) {
        match value {
            Value::Object(object) => {
                out.push(b'{');
                let mut entries: Vec<_> = object.iter().collect();
                entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
                for (index, (key, value)) in entries.into_iter().enumerate() {
                    if index != 0 {
                        out.push(b',');
                    }
                    out.extend(serde_json::to_vec(key).expect("JSON object key"));
                    out.push(b':');
                    write(value, out);
                }
                out.push(b'}');
            }
            Value::Array(array) => {
                out.push(b'[');
                for (index, value) in array.iter().enumerate() {
                    if index != 0 {
                        out.push(b',');
                    }
                    write(value, out);
                }
                out.push(b']');
            }
            _ => out.extend(serde_json::to_vec(value).expect("JSON scalar")),
        }
    }
    let mut output = Vec::new();
    write(value, &mut output);
    output
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|v| format!("{v:02x}")).collect()
}
fn combined(name: &str, source: &str, count: u32, schema: &[u8], values: &[u8]) -> String {
    let mut hash = stream(name, source, count);
    frame(&mut hash, schema);
    frame(&mut hash, values);
    hex(&hash.finalize())
}

fn exact(value: Option<&parameters::NativeValue>, extrema: bool) -> Result<Value, String> {
    use parameters::Value as Native;
    let Some(value) = value else {
        return Ok(Value::Null);
    };
    let data = match &value.value {
        Native::Empty => json!({"kind":"empty"}),
        Native::Integer(value) => json!({"kind":"integer","exact_decimal":value.to_string()}),
        Native::Float(value) if value.is_finite() => {
            json!({"kind":"float","bits_hex":format!("{:016X}",value.to_bits())})
        }
        Native::Float(_) => return Err("inconsistent finite-float metadata".into()),
        Native::NonFiniteFloat { bits } => {
            json!({"kind":"non_finite_float","bits_hex":format!("{bits:016X}")})
        }
        Native::Text(value) => json!({"kind":"text","value":value}),
        Native::OpaqueObject { .. } | Native::Unsupported { .. } if extrema => {
            return Err(
                "opaque or unsupported schema extrema cannot be digested completely".into(),
            );
        }
        Native::OpaqueObject { present } => json!({"kind":"opaque_object","present":present}),
        Native::Unsupported { kind } => json!({"kind":"unsupported","tag":kind}),
    };
    Ok(json!({"variant_tag":value.variant_tag,"data":data}))
}

struct Projection {
    schema: Value,
    values: Value,
    name: String,
    read_only: bool,
    readonly_observation: bool,
    opaque: bool,
}
fn project(observation: &Observation) -> Result<Projection, String> {
    let d = &observation.parameter;
    if matches!(d.kind, parameters::Kind::Unknown(_)) || d.raw.is_none() {
        return Err("unknown kind or missing raw parameter value".into());
    }
    let name = observation
        .dsp
        .as_ref()
        .map_or(d.name.as_str(), |dsp| dsp.name.as_str());
    for name in [name, d.name.as_str()] {
        if name.is_empty() || name.len() > parameters::MAX_NAME_BYTES || name.contains('\0') {
            return Err("empty, invalid or oversized canonical/local parameter name".into());
        }
    }
    if observation
        .dsp
        .as_ref()
        .is_some_and(|dsp| dsp.index != observation.index)
    {
        return Err("DSP canonical index does not match its observation".into());
    }
    let public = description(d);
    let writable = public["writable"]
        .as_bool()
        .expect("authoritative writable flag");
    let opaque = matches!(
        d.raw.as_ref().map(|value| &value.value),
        Some(parameters::Value::OpaqueObject { .. } | parameters::Value::Unsupported { .. })
    );
    let normalized = match d.normalized {
        parameters::Normalized::NotApplicable => json!({"kind":"not_applicable"}),
        parameters::Normalized::Finite(value) if value.is_finite() => {
            json!({"kind":"float32","bits_hex":format!("{:08X}",value.to_bits())})
        }
        parameters::Normalized::Finite(_) => {
            return Err("inconsistent finite-normalized metadata".into());
        }
        parameters::Normalized::NonFinite { bits } => {
            json!({"kind":"non_finite_float32","bits_hex":format!("{bits:08X}")})
        }
    };
    let formatter = match d.display_parser {
        parameters::DisplayParser::Unsupported => json!({"kind":"unsupported"}),
        parameters::DisplayParser::LiteralString => json!({"kind":"literal_string"}),
        parameters::DisplayParser::PlainNumeric => json!({"kind":"plain_numeric"}),
        parameters::DisplayParser::NativeFormatter { flags } => {
            json!({"kind":"native_formatter","flags":flags})
        }
    };
    let dsp = observation.dsp.as_ref().map(|dsp|json!({"index":dsp.index,"name":dsp.name,"tag":dsp.tag,"title":dsp.title,"folder":dsp.folder,"raw_flags":dsp.raw_flags}));
    Ok(Projection {
        schema: json!({"index":observation.index,"canonical_name":name,"dsp":dsp,"parameter":{
            "name":d.name,"tag":d.tag,"kind":format!("{:?}",d.kind),"enabled":d.enabled,"read_only":d.read_only,"writable":writable,
            "min":exact(d.min.as_ref(),true)?,"max":exact(d.max.as_ref(),true)?,"default":exact(d.default_value.as_ref(),true)?,
            "support":public["support"],
            "native_support":{"raw_number":d.support.raw_number,"normalized":d.support.normalized,"display_text":d.support.display_text},
            "display_parser":formatter}}),
        values: json!({"index":observation.index,"raw":exact(d.raw.as_ref(),false)?,"normalized":normalized,"display":d.display}),
        name: name.to_owned(),
        read_only: d.read_only,
        readonly_observation: d.read_only && !writable,
        opaque,
    })
}

fn scan<R: Reader>(
    source: &str,
    reader: &mut R,
    guard: &mut dyn FnMut() -> Result<(), String>,
) -> Result<Value, String> {
    guard()?;
    let count = reader.count()?;
    guard()?;
    if count > MAX_PARAMETERS {
        return Err("parameter digest count exceeds8192".into());
    }
    let mut names = HashSet::with_capacity(count as usize);
    let mut schema = stream("schema", source, count);
    let mut all_values = stream("all_value_records", source, count);
    let mut non_read_only = stream("non_read_only_value_records", source, count);
    let mut read_only_count = 0;
    let mut readonly_observation_count = 0;
    let mut opaque_count = 0;
    let mut offset = 0;
    while offset < count {
        guard()?;
        if reader.count()? != count {
            return Err("parameter count changed during digest".into());
        }
        let limit = CHUNK.min(count - offset);
        let observations = reader.chunk(offset, limit, count, guard)?;
        guard()?;
        if observations.len() != limit as usize {
            return Err("parameter digest page is incomplete".into());
        }
        for (position, observation) in observations.iter().enumerate() {
            guard()?;
            if observation.index != offset + position as u32 {
                return Err("parameter digest has missing, repeated or reordered indices".into());
            }
            let projection = project(observation)?;
            if !names.insert(projection.name) {
                return Err("parameter digest canonical names are not unique".into());
            }
            frame(&mut schema, &canonical_json(&projection.schema));
            let bytes = canonical_json(&projection.values);
            frame(&mut all_values, &bytes);
            if !projection.readonly_observation {
                frame(&mut non_read_only, &bytes);
            }
            read_only_count += u32::from(projection.read_only);
            readonly_observation_count += u32::from(projection.readonly_observation);
            opaque_count += u32::from(projection.opaque);
        }
        offset += limit;
        guard()?;
    }
    if reader.count()? != count {
        return Err("parameter count changed after digest".into());
    }
    guard()?;
    let schema = schema.finalize();
    let all_values = all_values.finalize();
    let non_read_only = non_read_only.finalize();
    let value = json!({
        "hash_format_version":FORMAT,"hash_algorithm":"sha256","hash_scope":"exposed_parameter_projection",
        "coverage":"fresh_sequential_indexed_native_parameter_observations","atomic_snapshot":false,
        "count":count,"readable_count":count,"read_only_count":read_only_count,"readonly_observation_count":readonly_observation_count,
        "non_read_only_count":count-readonly_observation_count,"excluded_opaque_count":opaque_count,"opaque_contents_verified":false,
        "opaque_exclusion_scope":"contents_only_records_remain_hashed","access_complete":true,"max_count":MAX_PARAMETERS,
        "schema_sha256":hex(&schema),
        "all_values_sha256":combined("all_values",source,count,&schema,&all_values),
        "non_read_only_values_sha256":combined("non_read_only_values",source,count,&schema,&non_read_only),
        "fields_covered":{
            "schema":["index","canonical_name","dsp.index/name/tag/title/folder/raw_flags","parameter.name/tag/kind/enabled/read_only/writable","parameter.min/max/default.typed_exact","parameter.support/native_support","parameter.display_parser/formatter_flags"],
            "values":["index","raw.variant_tag/data","normalized.kind/bits_hex","display"],
            "integer_encoding":"exact_decimal","float_encoding":"IEEE754_uppercase_hex_bits","opaque_values":"variant_tag/type/presence_only",
            "non_read_only_exclusion":"read_only=true AND writable=false"
        },
        "outcome":"observed",
    });
    guard()?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    fn native(value: parameters::Value) -> parameters::NativeValue {
        parameters::NativeValue {
            variant_tag: match value {
                parameters::Value::Integer(_) => 1,
                parameters::Value::Float(_) | parameters::Value::NonFiniteFloat { .. } => 2,
                parameters::Value::Text(_) => 3,
                parameters::Value::OpaqueObject { .. } => 4,
                parameters::Value::Empty => 0,
                parameters::Value::Unsupported { kind } => kind,
            },
            value,
        }
    }
    fn observation(index: u32, read_only: bool) -> Observation {
        Observation {
            index,
            dsp: None,
            parameter: parameters::Description {
                name: format!("parameter{index}"),
                tag: index as i32,
                kind: parameters::Kind::Float,
                enabled: true,
                read_only,
                raw: Some(native(parameters::Value::Float(1.0))),
                min: Some(native(parameters::Value::Float(0.0))),
                max: Some(native(parameters::Value::Float(10.0))),
                default_value: Some(native(parameters::Value::Float(1.0))),
                normalized: parameters::Normalized::Finite(0.1),
                display: "one".into(),
                display_parser: parameters::DisplayParser::NativeFormatter { flags: 0 },
                support: parameters::Support {
                    raw_number: !read_only,
                    normalized: !read_only,
                    display_text: !read_only,
                },
            },
        }
    }
    struct MockReader {
        observations: RefCell<Vec<Observation>>,
        reads: Cell<u32>,
        count_calls: u32,
        count_change: Option<u32>,
        fail: bool,
        incomplete: bool,
    }
    impl MockReader {
        fn new(records: Vec<Observation>) -> Self {
            Self {
                observations: RefCell::new(records),
                reads: Cell::new(0),
                count_calls: 0,
                count_change: None,
                fail: false,
                incomplete: false,
            }
        }
        fn digest(&mut self) -> Result<Value, String> {
            scan("controller", self, &mut || Ok(()))
        }
    }
    impl Reader for MockReader {
        fn count(&mut self) -> Result<u32, String> {
            self.count_calls += 1;
            Ok(self.observations.borrow().len() as u32
                + u32::from(
                    self.count_change
                        .is_some_and(|when| self.count_calls >= when),
                ))
        }
        fn chunk(
            &mut self,
            offset: u32,
            limit: u32,
            _expected_count: u32,
            guard: &mut dyn FnMut() -> Result<(), String>,
        ) -> Result<Vec<Observation>, String> {
            guard()?;
            self.reads.set(self.reads.get() + limit);
            if self.fail {
                return Err("access failure".into());
            }
            let mut records =
                self.observations.borrow()[offset as usize..(offset + limit) as usize].to_vec();
            if self.incomplete {
                records.pop();
            }
            guard()?;
            Ok(records)
        }
    }
    fn hash<'a>(value: &'a Value, key: &str) -> &'a str {
        value[key].as_str().unwrap()
    }
    #[test]
    fn canonical_json_and_length_frames_match_the_documented_format_vector() {
        let first = json!({"z":[1,2],"a":"μ\0"});
        let second = json!({"a":"μ\0","z":[1,2]});
        assert_eq!(canonical_json(&first), canonical_json(&second));
        let mut hash = stream("schema", "dsp", 1);
        frame(&mut hash, &canonical_json(&first));
        assert_eq!(
            hex(&hash.finalize()),
            "3f4a4960cf26fc1f39122116e5afc39b8cdb36114fd707ac4eec36e14ef30cf3"
        );
    }

    #[test]
    fn fresh_scans_are_stable_and_writable_value_changes_change_both_value_hashes() {
        let mut reader = MockReader::new(vec![observation(0, false), observation(1, true)]);
        let first = reader.digest().unwrap();
        let repeated = reader.digest().unwrap();
        assert_eq!(first, repeated);
        assert_eq!(reader.reads.get(), 4);
        reader.observations.borrow_mut()[0].parameter.raw =
            Some(native(parameters::Value::Float(2.0)));
        let changed = reader.digest().unwrap();
        assert_eq!(reader.reads.get(), 6);
        assert_eq!(
            hash(&first, "schema_sha256"),
            hash(&changed, "schema_sha256")
        );
        for key in ["all_values_sha256", "non_read_only_values_sha256"] {
            assert_ne!(hash(&first, key), hash(&changed, key));
        }
        assert_eq!(first["readable_count"], 2);
        assert_eq!(first["readonly_observation_count"], 1);
    }
    #[test]
    fn only_proven_readonly_records_are_omitted_from_non_read_only_digest() {
        let mut disabled = observation(2, false);
        disabled.parameter.enabled = false;
        disabled.parameter.support = parameters::Support {
            raw_number: false,
            normalized: false,
            display_text: false,
        };
        let mut opaque = observation(3, false);
        opaque.parameter.kind = parameters::Kind::Image;
        opaque.parameter.raw = Some(native(parameters::Value::OpaqueObject { present: true }));
        opaque.parameter.normalized = parameters::Normalized::NotApplicable;
        opaque.parameter.support = parameters::Support {
            raw_number: false,
            normalized: false,
            display_text: false,
        };
        let mut reader = MockReader::new(vec![
            observation(0, false),
            observation(1, true),
            disabled,
            opaque,
        ]);
        let original = reader.digest().unwrap();
        assert_eq!(original["excluded_opaque_count"], 1);
        assert_eq!(original["non_read_only_count"], 3);
        assert_eq!(original["opaque_contents_verified"], false);
        reader.observations.borrow_mut()[1].parameter.raw =
            Some(native(parameters::Value::Float(3.0)));
        let readonly = reader.digest().unwrap();
        assert_ne!(original["all_values_sha256"], readonly["all_values_sha256"]);
        assert_eq!(
            original["non_read_only_values_sha256"],
            readonly["non_read_only_values_sha256"]
        );
        reader.observations.borrow_mut()[2].parameter.raw =
            Some(native(parameters::Value::Float(3.0)));
        let disabled = reader.digest().unwrap();
        assert_ne!(
            readonly["non_read_only_values_sha256"],
            disabled["non_read_only_values_sha256"]
        );
        reader.observations.borrow_mut()[3].parameter.raw =
            Some(native(parameters::Value::OpaqueObject { present: false }));
        let opaque = reader.digest().unwrap();
        assert_ne!(
            disabled["non_read_only_values_sha256"],
            opaque["non_read_only_values_sha256"]
        );
    }
    #[test]
    fn every_schema_change_binds_both_value_hashes_and_preserves_dsp_local_identity() {
        let mut source = observation(0, false);
        source.dsp = Some(fender_native_dsp_api::Parameter {
            index: 0,
            tag: 900,
            name: "canonical.parameter".into(),
            title: "Canonical title".into(),
            folder: 7,
            raw_flags: 42,
        });
        let projection = project(&source).unwrap();
        assert_eq!(projection.schema["dsp"]["tag"], 900);
        assert_eq!(projection.schema["parameter"]["tag"], 0);
        assert_eq!(projection.schema["canonical_name"], "canonical.parameter");
        let original = MockReader::new(vec![source.clone()]).digest().unwrap();
        let mut variants = Vec::new();
        let mut changed = source.clone();
        changed.parameter.min = Some(native(parameters::Value::Float(f64::from_bits(1))));
        variants.push(changed);
        let mut changed = source.clone();
        changed.parameter.display_parser = parameters::DisplayParser::NativeFormatter { flags: 2 };
        variants.push(changed);
        let mut changed = source.clone();
        changed.dsp.as_mut().unwrap().tag = 901;
        variants.push(changed);
        let mut changed = source.clone();
        changed.parameter.tag = 99;
        variants.push(changed);
        let mut changed = source;
        changed.parameter.read_only = true;
        variants.push(changed);
        for changed in variants {
            let changed = MockReader::new(vec![changed]).digest().unwrap();
            for key in [
                "schema_sha256",
                "all_values_sha256",
                "non_read_only_values_sha256",
            ] {
                assert_ne!(original[key], changed[key]);
            }
        }
    }
    #[test]
    fn exact_float_integer_and_normalized_bits_survive_the_projection() {
        let mut source = observation(0, false);
        source.parameter.raw = Some(native(parameters::Value::Float(-0.0)));
        let projection = project(&source).unwrap();
        assert_eq!(
            projection.values["raw"]["data"]["bits_hex"],
            "8000000000000000"
        );
        let first = MockReader::new(vec![source.clone()]).digest().unwrap();
        source.parameter.raw = Some(native(parameters::Value::Float(f64::from_bits(
            0x8000000000000001,
        ))));
        let changed = MockReader::new(vec![source.clone()]).digest().unwrap();
        assert_ne!(first["all_values_sha256"], changed["all_values_sha256"]);
        source.parameter.raw = Some(native(parameters::Value::Float(f64::from_bits(
            0x3FF0000000000000,
        ))));
        let precise = MockReader::new(vec![source.clone()]).digest().unwrap();
        source.parameter.raw = Some(native(parameters::Value::Float(f64::from_bits(
            0x3FF0000000000001,
        ))));
        let one_ulp = MockReader::new(vec![source.clone()]).digest().unwrap();
        assert_ne!(precise["all_values_sha256"], one_ulp["all_values_sha256"]);
        source.parameter.normalized = parameters::Normalized::Finite(f32::from_bits(1));
        let normalized = MockReader::new(vec![source.clone()]).digest().unwrap();
        assert_ne!(
            changed["all_values_sha256"],
            normalized["all_values_sha256"]
        );
        source.parameter.raw = Some(native(parameters::Value::Integer(i64::MIN)));
        let exact = project(&source).unwrap();
        assert_eq!(
            exact.values["raw"]["data"]["exact_decimal"],
            "-9223372036854775808"
        );
        assert_eq!(exact.schema["parameter"]["writable"], false);
        let integer = MockReader::new(vec![source.clone()]).digest().unwrap();
        source.parameter.raw = Some(native(parameters::Value::Integer(i64::MIN + 1)));
        let adjacent = MockReader::new(vec![source]).digest().unwrap();
        assert_ne!(
            integer["non_read_only_values_sha256"],
            adjacent["non_read_only_values_sha256"]
        );
    }
    #[test]
    fn incomplete_unknown_duplicate_and_changed_counts_never_return_a_digest() {
        let mut oversized =
            MockReader::new(vec![observation(0, false); MAX_PARAMETERS as usize + 1]);
        assert!(oversized.digest().is_err());
        assert_eq!(oversized.reads.get(), 0);
        for kind in 0..7 {
            let mut reader = MockReader::new(vec![observation(0, false), observation(1, false)]);
            match kind {
                0 => reader.fail = true,
                1 => reader.incomplete = true,
                2 => reader.count_change = Some(3),
                3 => reader.observations.borrow_mut()[1].parameter.name = "parameter0".into(),
                4 => reader.observations.borrow_mut()[1].index = 0,
                5 => {
                    reader.observations.borrow_mut()[1].parameter.kind =
                        parameters::Kind::Unknown(77)
                }
                _ => reader.observations.borrow_mut()[1].parameter.raw = None,
            }
            assert!(reader.digest().is_err(), "failure kind{kind}");
        }
        for invalid in [
            parameters::Value::OpaqueObject { present: true },
            parameters::Value::Unsupported { kind: 8 },
        ] {
            for position in 0..3 {
                let mut record = observation(0, false);
                match position {
                    0 => record.parameter.min = Some(native(invalid.clone())),
                    1 => record.parameter.max = Some(native(invalid.clone())),
                    _ => record.parameter.default_value = Some(native(invalid.clone())),
                };
                assert!(MockReader::new(vec![record]).digest().is_err());
            }
        }
    }
    #[test]
    fn full4262_index_scan_is_bounded_complete_and_domain_separated() {
        let records = (0..4262)
            .map(|index| observation(index, index % 2 == 0))
            .collect();
        let mut reader = MockReader::new(records);
        let value = reader.digest().unwrap();
        assert_eq!(value["count"], 4262);
        assert_eq!(value["readable_count"], 4262);
        assert_eq!(reader.reads.get(), 4262);
        assert_eq!(value["readonly_observation_count"], 2131);
        assert_eq!(value["access_complete"], true);
        for key in [
            "schema_sha256",
            "all_values_sha256",
            "non_read_only_values_sha256",
        ] {
            assert_eq!(hash(&value, key).len(), 64);
        }
        let other = scan("dsp", &mut reader, &mut || Ok(())).unwrap();
        assert_ne!(value["schema_sha256"], other["schema_sha256"]);
        let empty = MockReader::new(vec![]).digest().unwrap();
        assert_eq!(empty["count"], 0);
        assert_eq!(empty["access_complete"], true);
    }
    #[test]
    fn request_is_fixed_explicit_and_readonly() {
        let valid = json!({"operation":"parameter_digest","source":"dsp","expected_instance":uid(&SERVICE_CID)});
        for source in ["dsp", "controller"] {
            let mut request = valid.clone();
            request["source"] = json!(source);
            assert!(
                serde_json::from_value::<Request>(request)
                    .unwrap()
                    .validate()
                    .is_ok()
            );
        }
        for extra in [
            "offset",
            "limit",
            "value",
            "values",
            "source_path",
            "param",
            "bytes",
            "include_nodes",
        ] {
            let mut value = valid.clone();
            value[extra] = json!(1);
            assert!(serde_json::from_value::<Request>(value).is_err());
        }
        for remove in ["source", "expected_instance"] {
            let mut value = valid.clone();
            value.as_object_mut().unwrap().remove(remove);
            assert!(
                serde_json::from_value::<Request>(value)
                    .and_then(|request| request.validate().map_err(serde::de::Error::custom))
                    .is_err()
            );
        }
        let mut value = valid;
        value["source"] = json!("auto");
        assert!(
            serde_json::from_value::<Request>(value)
                .unwrap()
                .validate()
                .is_err()
        );
    }
    #[test]
    fn lookup_read_release_and_deadline_failures_cannot_publish_hashes() {
        struct Owner<'a>(&'a Cell<bool>);
        impl Drop for Owner<'_> {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        let released = Cell::new(false);
        let d = observation(0, false).parameter;
        let result = observe(|| Ok(Owner(&released)), |_| Ok(d.clone()), &mut || {
            if released.get() {
                Err("destructor changed context".into())
            } else {
                Ok(())
            }
        });
        assert!(result.is_err());
        assert!(released.get());
        released.set(false);
        let looked_up = Cell::new(false);
        let read_called = Cell::new(false);
        let result = observe(
            || {
                looked_up.set(true);
                Ok(Owner(&released))
            },
            |_| {
                read_called.set(true);
                Ok(d.clone())
            },
            &mut || {
                if looked_up.get() {
                    Err("lookup reentry".into())
                } else {
                    Ok(())
                }
            },
        );
        assert!(result.is_err());
        assert!(released.get());
        assert!(!read_called.get());
        released.set(false);
        let result = observe(
            || Ok(Owner(&released)),
            |_| Err("native read failed".into()),
            &mut || Ok(()),
        );
        assert!(result.is_err());
        assert!(released.get());
        let reads = Cell::new(0);
        let result = observe(
            || {
                reads.set(1);
                Ok(Owner(&released))
            },
            |_| {
                reads.set(2);
                Ok(d.clone())
            },
            &mut || Err("already stale".into()),
        );
        assert!(result.is_err());
        assert_eq!(reads.get(), 0);
        let mut reader = MockReader::new(vec![observation(0, false)]);
        let guards = Cell::new(0);
        let result = scan("controller", &mut reader, &mut || {
            guards.set(guards.get() + 1);
            if guards.get() > 6 {
                Err("two second deadline".into())
            } else {
                Ok(())
            }
        });
        assert!(result.is_err());
        let mut reply = Reply::unbound(
            MockReader::new(vec![observation(0, false)])
                .digest()
                .unwrap(),
        );
        reply.invalidate("context changed before publication".into());
        assert_eq!(reply.value["ok"], false);
        assert_eq!(reply.value["access_complete"], false);
        for key in [
            "schema_sha256",
            "all_values_sha256",
            "non_read_only_values_sha256",
        ] {
            assert!(reply.value.get(key).is_none());
        }
    }
}
