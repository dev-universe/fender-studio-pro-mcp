use super::{Availability, Catalog, Entry, Error, Owned, ffi, guard};
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use sha2::{Digest, Sha256};
use std::{
    cell::Cell,
    ffi::{CStr, c_char, c_void},
    fs::{File, Metadata, OpenOptions, symlink_metadata},
    io::{self, Read, Seek, SeekFrom},
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};
use zip::{
    ZipArchive,
    read::{ArchiveOffset, Config},
};

const CV_CLASS: &str = "EDCD22A5-A01E-4EA6-9FE4-2BB1AB243E85";
const CV_CLASS_BRACED: &str = "{EDCD22A5-A01E-4EA6-9FE4-2BB1AB243E85}";
const MAX_PATH: usize = 2048;
const MAX_ARCHIVE: u64 = 1024 * 1024;
const MAX_ENTRIES: usize = 16;
const MAX_META: u64 = 64 * 1024;
const MAX_SETTINGS: u64 = 256 * 1024;
const MAX_XML_DEPTH: usize = 32;
const SETTINGS: &str = "data.fxpreset";

pub struct CvSettingsInspection {
    index: u32,
    file: FileWitness,
    byte_count: u64,
    hash: String,
}

impl CvSettingsInspection {
    pub fn byte_count(&self) -> u64 {
        self.byte_count
    }
    pub fn sha256(&self) -> &str {
        &self.hash
    }
    pub fn check<G: FnMut() -> Result<(), String>>(
        &self,
        catalog: &Catalog,
        g: &mut G,
    ) -> Result<(), Error> {
        if catalog.cv_preset_path(self.index, g)? != self.file.path {
            return Err(Error::Changed);
        }
        self.file.check_named()?;
        catalog.borrowed_target()?;
        guard(g)
    }
}

impl Catalog {
    pub fn inspect_cv_settings<G: FnMut() -> Result<(), String>>(
        &self,
        index: u32,
        g: &mut G,
    ) -> Result<CvSettingsInspection, Error> {
        let path = self.cv_preset_path(index, g)?;
        let (file, mut handle) = FileWitness::open(path)?;
        let report = (|| {
            let (byte_count, hash) = archive_settings(&mut handle, file.stamp.len)?;
            file.check_open(&handle)?;
            Ok(CvSettingsInspection {
                index,
                file,
                byte_count,
                hash,
            })
        })();
        drop(handle);
        guard(g)?;
        let report = report?;
        report.check(self, g)?;
        Ok(report)
    }

    fn cv_preset_entry<G: FnMut() -> Result<(), String>>(
        &self,
        index: u32,
        g: &mut G,
    ) -> Result<&Entry, Error> {
        self.check(g)?;
        let entry = self.entries.get(index as usize).ok_or(Error::OutOfRange)?;
        if !matches!(self.class_key.as_str(), CV_CLASS | CV_CLASS_BRACED)
            || !matches!(entry.info.class_key.as_str(), CV_CLASS | CV_CLASS_BRACED)
            || !matches!(
                entry.info.availability,
                Availability::LocalReadOnly | Availability::LocalWritable
            )
        {
            return Err(Error::Unavailable(
                "preset is not an exact available local CV catalog entry",
            ));
        }
        self.check_entry(entry, g)?;
        Ok(entry)
    }

    fn cv_preset_path<G: FnMut() -> Result<(), String>>(
        &self,
        index: u32,
        g: &mut G,
    ) -> Result<PathBuf, Error> {
        let entry = self.cv_preset_entry(index, g)?;
        let url = entry
            .url
            .as_ref()
            .ok_or(Error::Unavailable("preset URL is unavailable"))?;
        let path = plain_preset_path(url, g)?;
        self.check_entry(entry, g)?;
        self.borrowed_target()?;
        guard(g)?;
        Ok(path)
    }
}

fn plain_preset_path<G: FnMut() -> Result<(), String>>(
    url: &Owned,
    g: &mut G,
) -> Result<PathBuf, Error> {
    let has_parameters: unsafe extern "C" fn(*mut c_void) -> u8 =
        unsafe { ffi::method(url.raw(), 41) };
    let plain = unsafe { has_parameters(url.raw()) } == 0;
    guard(g)?;
    if !plain {
        return Err(Error::InvalidData("preset URL has parameters"));
    }
    let copy: unsafe extern "C" fn(*mut c_void, *mut c_char, i32) -> u8 =
        unsafe { ffi::method(url.raw(), 27) };
    let mut buffer = [0x7f_u8; MAX_PATH];
    let copied = unsafe { copy(url.raw(), buffer.as_mut_ptr().cast(), MAX_PATH as i32) };
    guard(g)?;
    if copied != 1 {
        return Err(Error::Unavailable("preset local file path unavailable"));
    }
    let bytes = CStr::from_bytes_until_nul(&buffer)
        .map_err(|_| Error::InvalidData("preset local file path is not terminated"))?
        .to_bytes();
    if !bytes.starts_with(b"/")
        || !bytes.ends_with(b".preset")
        || bytes[1..]
            .split(|byte| *byte == b'/')
            .any(|part| part.is_empty() || part == b"." || part == b"..")
    {
        return Err(Error::InvalidData(
            "preset local file path is not a normalized .preset file",
        ));
    }
    Ok(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
}

#[derive(PartialEq, Eq)]
struct FileStamp {
    dev: u64,
    ino: u64,
    len: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl FileStamp {
    fn capture(metadata: Metadata) -> Result<Self, Error> {
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_ARCHIVE {
            return Err(Error::InvalidData("preset must be a bounded regular file"));
        }
        Ok(Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
            len: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        })
    }
}

struct FileWitness {
    path: PathBuf,
    stamp: FileStamp,
}
impl FileWitness {
    fn open(path: PathBuf) -> Result<(Self, File), Error> {
        let stamp = named_stamp(&path)?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)
            .map_err(|_| Error::Unavailable("preset regular file could not be opened"))?;
        let witness = Self { path, stamp };
        witness.check_open(&file)?;
        witness.check_named()?;
        Ok((witness, file))
    }
    fn check_open(&self, file: &File) -> Result<(), Error> {
        let stamp = file
            .metadata()
            .map_err(|_| Error::Changed)
            .and_then(FileStamp::capture)?;
        self.check_stamp(stamp)
    }
    fn check_named(&self) -> Result<(), Error> {
        self.check_stamp(named_stamp(&self.path)?)
    }
    fn check_stamp(&self, stamp: FileStamp) -> Result<(), Error> {
        if stamp == self.stamp {
            Ok(())
        } else {
            Err(Error::Changed)
        }
    }
}
fn named_stamp(path: &Path) -> Result<FileStamp, Error> {
    FileStamp::capture(
        symlink_metadata(path)
            .map_err(|_| Error::Unavailable("preset regular file unavailable"))?,
    )
}

struct BoundedReader<'a, R> {
    inner: R,
    end: u64,
    pos: u64,
    structural_start: &'a Cell<u64>,
}
impl<R: Read + Seek> Read for BoundedReader<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = (self.end - self.pos).min(buffer.len() as u64) as usize;
        let masked = self
            .structural_start
            .get()
            .saturating_sub(self.pos)
            .min(count as u64) as usize;
        if masked > 0 {
            buffer[..masked].fill(0);
            self.seek(SeekFrom::Current(masked as i64))?;
        }
        let done = self.inner.read(&mut buffer[masked..count])?;
        self.pos += done as u64;
        Ok(masked + done)
    }
}
impl<R: Seek> Seek for BoundedReader<'_, R> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let pos = match from {
            SeekFrom::Start(pos) => Some(pos),
            SeekFrom::Current(delta) => self.pos.checked_add_signed(delta),
            SeekFrom::End(delta) => self.end.checked_add_signed(delta),
        }
        .filter(|pos| *pos <= self.end)
        .ok_or(io::ErrorKind::InvalidInput)?;
        if self.inner.seek(SeekFrom::Start(pos))? != pos {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.pos = pos;
        Ok(pos)
    }
}

fn central_directory<R: Read + Seek>(
    reader: &mut BoundedReader<'_, R>,
) -> Result<(u64, usize), Error> {
    let invalid = Error::InvalidData("unsupported preset ZIP footer layout");
    let mut footer = [0; 22];
    let footer_start = reader
        .end
        .checked_sub(footer.len() as u64)
        .ok_or_else(|| invalid.clone())?;
    reader
        .seek(SeekFrom::Start(footer_start))
        .map_err(|_| invalid.clone())?;
    reader
        .read_exact(&mut footer)
        .map_err(|_| invalid.clone())?;
    let entry_count = u16::from_le_bytes([footer[10], footer[11]]) as usize;
    let size = u32::from_le_bytes(footer[12..16].try_into().expect("fixed footer field")) as u64;
    let start = u32::from_le_bytes(footer[16..20].try_into().expect("fixed footer field")) as u64;
    if footer[..4] != *b"PK\x05\x06"
        || footer[4..8] != [0; 4]
        || footer[8..10] != footer[10..12]
        || footer[20..22] != [0; 2]
        || !(1..=MAX_ENTRIES).contains(&entry_count)
        || size < entry_count as u64 * 46
        || start + size != footer_start
    {
        return Err(invalid);
    }
    Ok((start, entry_count))
}

fn archive_settings<R: Read + Seek>(reader: R, end: u64) -> Result<(u64, String), Error> {
    let structural_start = Cell::new(0);
    let mut reader = BoundedReader {
        inner: reader,
        end,
        pos: 0,
        structural_start: &structural_start,
    };
    let (directory, count) = central_directory(&mut reader)?;
    // A ZIP reader scans backwards for the directory marker. Keep that scan
    // inside the known directory/footer region, outside every member body.
    structural_start.set(directory);
    let mut archive = ZipArchive::with_config(
        Config {
            archive_offset: ArchiveOffset::Known(0),
        },
        reader,
    )
    .map_err(|_| Error::InvalidData("unsupported preset ZIP"))?;
    structural_start.set(0);
    if archive.central_directory_start() != directory || archive.len() != count {
        return Err(Error::InvalidData(
            "preset ZIP member count or directory changed",
        ));
    }
    if archive.has_duplicate_names()
        || archive.file_names_raw().any(|name| {
            name.is_empty()
                || name.len() > 128
                || name == b"."
                || name == b".."
                || name
                    .iter()
                    .any(|byte| byte.is_ascii_control() || matches!(*byte, b'/' | b'\\' | b':'))
        })
    {
        return Err(Error::InvalidData(
            "preset ZIP has unsafe, duplicate or excessive members",
        ));
    }
    archive
        .validate_header_consistency()
        .map_err(|_| Error::InvalidData("inconsistent preset ZIP headers"))?;
    let metadata = read_member(&mut archive, "metainfo.xml", MAX_META)?;
    validate_xml(&metadata, Format::Metadata)?;
    let settings = read_member(&mut archive, SETTINGS, MAX_SETTINGS)?;
    validate_xml(&settings, Format::Settings)?;
    Ok((
        settings.len() as u64,
        format!("{:x}", Sha256::digest(settings)),
    ))
}

fn read_member<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    limit: u64,
) -> Result<Vec<u8>, Error> {
    let mut member = archive
        .by_name(name)
        .map_err(|_| Error::InvalidData("required preset ZIP member unavailable"))?;
    let size = member.size();
    if !member.is_file() || size == 0 || size > limit {
        return Err(Error::InvalidData(
            "required preset ZIP member has invalid type or size",
        ));
    }
    let mut bytes = Vec::with_capacity(size as usize);
    member
        .by_ref()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::InvalidData("required preset ZIP member could not be read"))?;
    if bytes.len() as u64 != size {
        return Err(Error::InvalidData(
            "required preset ZIP member size changed",
        ));
    }
    Ok(bytes)
}

#[derive(Clone, Copy)]
enum Format {
    Metadata,
    Settings,
}
fn validate_xml(bytes: &[u8], format: Format) -> Result<(), Error> {
    let invalid = Error::InvalidData("unsupported CV preset XML structure");
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    std::str::from_utf8(bytes).map_err(|_| invalid.clone())?;
    let mut reader = Reader::from_reader(bytes);
    let fields = match format {
        Format::Metadata => [
            (b"Class:ID".as_slice(), CV_CLASS_BRACED.as_bytes()),
            (b"Class:Name", b"CV Instrument"),
            (b"Preset:DataFile", SETTINGS.as_bytes()),
        ],
        Format::Settings => [
            (b"cid".as_slice(), CV_CLASS_BRACED.as_bytes()),
            (b"version", b"2"),
            (b"", b""),
        ],
    };
    let required = if matches!(format, Format::Metadata) {
        3
    } else {
        2
    };
    let root = match format {
        Format::Metadata => b"MetaInformation".as_slice(),
        Format::Settings => b"AudioEffectPreset".as_slice(),
    };
    let mut seen = [false; 3];
    let mut root_seen = false;
    let mut depth = 0_usize;
    loop {
        let event = reader.read_event().map_err(|_| invalid.clone())?;
        match event {
            Event::Start(ref tag) | Event::Empty(ref tag) => {
                if depth == 0 {
                    if root_seen || tag.name().as_ref() != root || matches!(event, Event::Empty(_))
                    {
                        return Err(invalid);
                    }
                    root_seen = true;
                }
                if matches!(format, Format::Settings) && depth == 0 {
                    for (index, (key, value)) in fields[..required].iter().enumerate() {
                        seen[index] = xml_attr(tag, key)? == Some(*value);
                    }
                } else if matches!(format, Format::Metadata)
                    && tag.name().as_ref() == b"Attribute"
                    && let Some(id) = xml_attr(tag, b"id")?
                    && let Some((index, (_, value))) =
                        fields.iter().enumerate().find(|(_, (key, _))| *key == id)
                {
                    if depth != 1 || seen[index] || xml_attr(tag, b"value")? != Some(*value) {
                        return Err(invalid);
                    }
                    seen[index] = true;
                }
                if matches!(event, Event::Start(_)) {
                    depth += 1;
                    if depth > MAX_XML_DEPTH {
                        return Err(invalid);
                    }
                }
            }
            Event::End(_) => depth = depth.checked_sub(1).ok_or_else(|| invalid.clone())?,
            Event::DocType(_) => return Err(invalid),
            Event::Text(text) if depth == 0 && text.iter().any(|c| !c.is_ascii_whitespace()) => {
                return Err(invalid);
            }
            Event::CData(_) | Event::GeneralRef(_) if depth == 0 => return Err(invalid),
            Event::Eof => break,
            _ => {}
        }
    }
    if !root_seen || depth != 0 || seen[..required].contains(&false) {
        return Err(invalid);
    }
    Ok(())
}
fn xml_attr<'a>(tag: &'a BytesStart<'_>, key: &[u8]) -> Result<Option<&'a [u8]>, Error> {
    let mut value = None;
    for attribute in tag.attributes() {
        let attribute =
            attribute.map_err(|_| Error::InvalidData("invalid CV preset XML attribute"))?;
        if attribute.key.as_ref() == key {
            value = match attribute.value {
                std::borrow::Cow::Borrowed(bytes) => Some(bytes),
                std::borrow::Cow::Owned(_) => {
                    return Err(Error::InvalidData(
                        "unexpected CV preset XML attribute encoding",
                    ));
                }
            };
        }
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Cursor, Write},
        ops::Range,
    };
    use zip::{ZipWriter, write::SimpleFileOptions};

    fn metadata() -> String {
        format!(
            r#"<MetaInformation><Attribute id="Class:ID" value="{CV_CLASS_BRACED}"/><Attribute id="Class:Name" value="CV Instrument"/><Attribute id="Preset:DataFile" value="{SETTINGS}"/></MetaInformation>"#
        )
    }
    fn settings() -> String {
        format!(
            r#"<AudioEffectPreset cid="{CV_CLASS_BRACED}" version="2"><ComponentData/></AudioEffectPreset>"#
        )
    }
    fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, payload) in entries {
            zip.start_file(
                *name,
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
            zip.write_all(payload).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }
    fn inspect(bytes: &[u8]) -> Result<(u64, String), Error> {
        archive_settings(Cursor::new(bytes), bytes.len() as u64)
    }

    struct GuardedReader<'a> {
        inner: Cursor<&'a [u8]>,
        forbidden: Range<u64>,
    }
    impl Read for GuardedReader<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let start = self.inner.position();
            let end = (start + buffer.len() as u64).min(self.inner.get_ref().len() as u64);
            assert!(
                start == end || end <= self.forbidden.start || start >= self.forbidden.end,
                "settings inspection read bytes from PortSetup.data"
            );
            self.inner.read(buffer)
        }
    }
    impl Seek for GuardedReader<'_> {
        fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
            self.inner.seek(from)
        }
    }

    #[test]
    fn reads_only_the_primary_and_metadata_without_reading_port_setup_payload() {
        let route = b"hidden route content of a valid ZIP";
        let settings = settings();
        let bytes = zip(&[
            (SETTINGS, settings.as_bytes()),
            ("PortSetup.data", route),
            ("metainfo.xml", metadata().as_bytes()),
        ]);
        let start = bytes
            .windows(route.len())
            .position(|window| window == route)
            .unwrap() as u64;
        let reader = GuardedReader {
            inner: Cursor::new(&bytes),
            forbidden: start..start + route.len() as u64,
        };
        assert_eq!(
            archive_settings(reader, bytes.len() as u64).unwrap(),
            (
                settings.len() as u64,
                format!("{:x}", Sha256::digest(&settings))
            )
        );
    }

    #[test]
    fn refuses_traversal_duplicate_members_and_wrong_metadata_or_primary_class_version() {
        let m = metadata();
        let s = settings();
        for unsafe_name in [
            "../PortSetup.data",
            "/PortSetup.data",
            "nested/PortSetup.data",
            "a\\b",
        ] {
            let bytes = zip(&[
                (SETTINGS, s.as_bytes()),
                ("metainfo.xml", m.as_bytes()),
                (unsafe_name, b"probe"),
            ]);
            assert!(inspect(&bytes).is_err(), "{unsafe_name}");
        }
        let mut duplicate = zip(&[
            (SETTINGS, s.as_bytes()),
            ("metainfo.xml", m.as_bytes()),
            ("data.fxpreseX", b"extra"),
        ]);
        let marker = b"data.fxpreseX";
        let offsets: Vec<_> = duplicate
            .windows(marker.len())
            .enumerate()
            .filter_map(|(offset, name)| (name == marker).then_some(offset))
            .collect();
        assert_eq!(offsets.len(), 2);
        for offset in offsets {
            duplicate[offset..offset + marker.len()].copy_from_slice(SETTINGS.as_bytes());
        }
        assert!(inspect(&duplicate).is_err());
        for invalid in [
            m.replace(CV_CLASS_BRACED, "wrong"),
            m.replace("CV Instrument", "Other"),
            m.replace(SETTINGS, "PortSetup.data"),
            m.replace(
                "</MetaInformation>",
                &format!(
                    r#"<Attribute id="Preset:DataFile" value="{SETTINGS}"/></MetaInformation>"#
                ),
            ),
            m.replace("</MetaInformation>", ""),
        ] {
            assert!(
                inspect(&zip(&[
                    (SETTINGS, s.as_bytes()),
                    ("metainfo.xml", invalid.as_bytes())
                ]))
                .is_err()
            );
        }
        for invalid in [
            s.replace(CV_CLASS_BRACED, "wrong"),
            s.replace("version=\"2\"", "version=\"1\""),
            s.replace("AudioEffectPreset", "OtherPreset"),
            s.repeat(2),
        ] {
            assert!(
                inspect(&zip(&[
                    (SETTINGS, invalid.as_bytes()),
                    ("metainfo.xml", m.as_bytes())
                ]))
                .is_err()
            );
        }
    }

    #[test]
    fn caps_the_declared_entry_count_and_inflated_selected_member() {
        let mut excessive = zip(&[
            (SETTINGS, settings().as_bytes()),
            ("metainfo.xml", metadata().as_bytes()),
        ]);
        let len = excessive.len();
        excessive[len - 14..len - 12].copy_from_slice(&((MAX_ENTRIES + 1) as u16).to_le_bytes());
        excessive[len - 12..len - 10].copy_from_slice(&((MAX_ENTRIES + 1) as u16).to_le_bytes());
        assert!(inspect(&excessive).is_err());
        let too_big = vec![b'x'; (MAX_SETTINGS + 1) as usize];
        assert!(
            inspect(&zip(&[
                (SETTINGS, &too_big),
                ("metainfo.xml", metadata().as_bytes())
            ]))
            .is_err()
        );
    }

    #[test]
    fn file_witness_rejects_symlinks_and_replaced_paths_after_closing_the_fd() {
        let folder =
            std::env::temp_dir().join(format!("fender-cv-settings-test-{}", std::process::id()));
        std::fs::create_dir(&folder).unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                std::fs::remove_dir_all(&self.0).unwrap();
            }
        }
        let _cleanup = Cleanup(folder.clone());
        let path = folder.join("fixture.preset");
        let moved = folder.join("old.preset");
        let link = folder.join("link.preset");
        std::fs::write(&path, b"same size").unwrap();
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(FileWitness::open(link).is_err());
        let (witness, file) = FileWitness::open(path.clone()).unwrap();
        witness.check_open(&file).unwrap();
        drop(file);
        witness.check_named().unwrap();
        std::fs::rename(&path, moved).unwrap();
        std::fs::write(&path, b"same size").unwrap();
        assert_eq!(witness.check_named(), Err(Error::Changed));
    }
}
