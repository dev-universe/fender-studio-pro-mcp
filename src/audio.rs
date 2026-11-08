use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

use ebur128::{Channel, EbuR128, Mode};
use hound::{SampleFormat, WavReader};
use serde::Serialize;

const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_HEADER_BYTES: u64 = 1024 * 1024;
const MAX_HEADER_CHUNKS: usize = 1024;
const MAX_CHANNELS: u16 = 32;
const MIN_SAMPLE_RATE: u32 = 8_000;
const MAX_SAMPLE_RATE: u32 = 384_000;
const MAX_DURATION_SECONDS: u64 = 4 * 60 * 60;
const BLOCK_FRAMES: usize = 4096;

#[derive(Debug, Serialize)]
pub struct AudioAnalysis {
    pub source: &'static str,
    pub format: &'static str,
    pub status: SignalStatus,
    pub file_bytes: u64,
    pub sample_rate_hz: u32,
    pub channel_count: u16,
    pub bits_per_sample: u16,
    pub sample_format: &'static str,
    pub frames: u64,
    pub samples: u64,
    pub duration_seconds: f64,
    pub sample_peak_linear: f64,
    pub sample_peak_dbfs: Option<f64>,
    pub true_peak_dbtp: Option<f64>,
    pub rms_linear: f64,
    pub rms_dbfs: Option<f64>,
    pub crest_db: Option<f64>,
    pub integrated_lufs: Option<f64>,
    pub loudness_status: &'static str,
    pub clipping: ClippingAnalysis,
    pub channels: Vec<ChannelAnalysis>,
    pub stereo: Option<StereoAnalysis>,
    pub method: AnalysisMethod,
    pub limitations: Vec<&'static str>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalStatus {
    Empty,
    Silence,
    Measured,
}

#[derive(Debug, Serialize)]
pub struct ClippingAnalysis {
    pub samples_at_or_above_full_scale: u64,
    pub frames_at_or_above_full_scale: u64,
    pub definition: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ChannelAnalysis {
    pub index: u16,
    pub samples: u64,
    pub sample_peak_linear: f64,
    pub sample_peak_dbfs: Option<f64>,
    pub true_peak_dbtp: Option<f64>,
    pub rms_linear: f64,
    pub rms_dbfs: Option<f64>,
    pub crest_db: Option<f64>,
    pub samples_at_or_above_full_scale: u64,
}

#[derive(Debug, Serialize)]
pub struct StereoAnalysis {
    pub correlation: Option<f64>,
    pub correlation_definition: &'static str,
    pub mid_rms_linear: f64,
    pub side_rms_linear: f64,
    pub side_to_mid_db: Option<f64>,
    pub side_to_mid_status: &'static str,
    pub side_energy_fraction: Option<f64>,
    pub mid_side_definition: &'static str,
}

#[derive(Debug, Serialize)]
pub struct AnalysisMethod {
    pub decoder: &'static str,
    pub loudness: &'static str,
    pub true_peak: &'static str,
    pub loudness_channel_layout: &'static str,
    pub true_peak_tail_padding_frames: u16,
    pub rms_definition: &'static str,
}

pub fn analyze_file(path: &Path) -> Result<AudioAnalysis, String> {
    let mut file = File::open(path).map_err(|e| format!("cannot open audio file: {e}"))?;
    let file_bytes = validate_container(&mut file)?;
    let mut reader = WavReader::new(BufReader::new(file))
        .map_err(|e| format!("unsupported or invalid WAV: {e}"))?;
    let spec = reader.spec();
    if spec.channels > MAX_CHANNELS {
        return Err(format!(
            "audio analysis supports at most {MAX_CHANNELS} channels"
        ));
    }
    if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&spec.sample_rate) {
        return Err(format!(
            "sample rate must be {MIN_SAMPLE_RATE}..={MAX_SAMPLE_RATE} Hz"
        ));
    }
    if !reader.len().is_multiple_of(u32::from(spec.channels)) {
        return Err("WAV data does not contain whole interleaved frames".into());
    }
    let declared_frames = u64::from(reader.duration());
    if declared_frames > u64::from(spec.sample_rate) * MAX_DURATION_SECONDS {
        return Err("audio duration exceeds the four-hour analysis limit".into());
    }
    let integer = spec.sample_format == SampleFormat::Int;
    if (integer && !matches!(spec.bits_per_sample, 8 | 16 | 24 | 32))
        || (!integer && spec.bits_per_sample != 32)
    {
        return Err("supported WAV encodings are PCM 8/16/24/32-bit and float32".into());
    }
    let denominator = 2_f64.powi(i32::from(spec.bits_per_sample) - 1);
    let positive_full_scale = if integer {
        (denominator - 1.0) / denominator
    } else {
        1.0
    };
    let mut meter = SignalMeter::new(spec.channels, spec.sample_rate, positive_full_scale)?;
    if integer {
        meter.consume(
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|value| f64::from(value) / denominator)),
        )?;
    } else {
        meter.consume(reader.samples::<f32>().map(|sample| sample.map(f64::from)))?;
    }
    if meter.frames() != declared_frames {
        return Err("decoded frame count differs from the WAV header".into());
    }
    meter.finish(spec, file_bytes)
}

fn validate_container(file: &mut File) -> Result<u64, String> {
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("audio input must be a regular file".into());
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err("audio file exceeds the 2 GiB analysis limit".into());
    }
    let mut header = [0_u8; 12];
    file.read_exact(&mut header)
        .map_err(|_| "truncated WAV header".to_string())?;
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return Err("audio analysis currently accepts little-endian RIFF WAVE files".into());
    }
    let end = u64::from(u32::from_le_bytes(header[4..8].try_into().unwrap())) + 8;
    if end < 12 || end > metadata.len() {
        return Err("RIFF length exceeds the available file data".into());
    }
    let mut offset = 12_u64;
    for _ in 0..MAX_HEADER_CHUNKS {
        if offset + 8 > end || offset > MAX_HEADER_BYTES {
            return Err("WAV metadata exceeds its size or chunk limit".into());
        }
        let mut chunk = [0_u8; 8];
        file.read_exact(&mut chunk)
            .map_err(|_| "truncated WAV chunk header".to_string())?;
        let length = u64::from(u32::from_le_bytes(chunk[4..8].try_into().unwrap()));
        let data_end = offset
            .checked_add(8 + length)
            .ok_or_else(|| "WAV chunk length overflow".to_string())?;
        if data_end > end {
            return Err("WAV chunk exceeds the declared RIFF container".into());
        }
        if &chunk[0..4] == b"data" {
            file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
            return Ok(metadata.len());
        }
        let next = data_end + length % 2;
        if next > end {
            return Err("WAV metadata padding exceeds the declared RIFF container".into());
        }
        if length > MAX_HEADER_BYTES || next > MAX_HEADER_BYTES {
            return Err("WAV metadata before the audio data exceeds 1 MiB".into());
        }
        file.seek(SeekFrom::Start(next))
            .map_err(|e| e.to_string())?;
        offset = next;
    }
    Err("WAV contains too many chunks before its audio data".into())
}

#[derive(Default)]
struct ChannelMeter {
    peak: f64,
    square_sum: f64,
    samples: u64,
    clipped: u64,
}

#[derive(Default)]
struct StereoMeter {
    frames: u64,
    left_mean: f64,
    right_mean: f64,
    left_variance_sum: f64,
    right_variance_sum: f64,
    covariance_sum: f64,
    mid_square_sum: f64,
    side_square_sum: f64,
}

impl StereoMeter {
    fn add(&mut self, left: f64, right: f64) {
        self.frames += 1;
        let count = self.frames as f64;
        let left_delta = left - self.left_mean;
        let right_delta = right - self.right_mean;
        self.left_mean += left_delta / count;
        self.right_mean += right_delta / count;
        self.left_variance_sum += left_delta * (left - self.left_mean);
        self.right_variance_sum += right_delta * (right - self.right_mean);
        self.covariance_sum += left_delta * (right - self.right_mean);
        self.mid_square_sum += ((left + right) * 0.5).powi(2);
        self.side_square_sum += ((left - right) * 0.5).powi(2);
    }

    fn finish(self) -> StereoAnalysis {
        let mid = rms(self.mid_square_sum, self.frames);
        let side = rms(self.side_square_sum, self.frames);
        let correlation_denominator = (self.left_variance_sum * self.right_variance_sum).sqrt();
        let correlation = if correlation_denominator > 0.0 {
            finite(self.covariance_sum / correlation_denominator).map(|v| v.clamp(-1.0, 1.0))
        } else {
            None
        };
        let energy = self.mid_square_sum + self.side_square_sum;
        StereoAnalysis {
            correlation,
            correlation_definition: "zero-lag Pearson correlation of the two original channels",
            mid_rms_linear: mid,
            side_rms_linear: side,
            side_to_mid_db: if mid > 0.0 && side > 0.0 {
                db(side / mid)
            } else {
                None
            },
            side_to_mid_status: if mid == 0.0 && side == 0.0 {
                "silent"
            } else if mid == 0.0 {
                "mid_silent"
            } else if side == 0.0 {
                "side_silent"
            } else {
                "finite"
            },
            side_energy_fraction: if energy > 0.0 {
                finite(self.side_square_sum / energy)
            } else {
                None
            },
            mid_side_definition: "mid=(L+R)/2, side=(L-R)/2; side energy fraction is not perceived stereo width",
        }
    }
}

struct SignalMeter {
    channels: Vec<ChannelMeter>,
    ebu: EbuR128,
    positive_full_scale: f64,
    samples: u64,
    clipped_frames: u64,
    frame_clipped: bool,
    left: f64,
    stereo: Option<StereoMeter>,
    block: Vec<f64>,
}

impl SignalMeter {
    fn new(channels: u16, rate: u32, positive_full_scale: f64) -> Result<Self, String> {
        let mode = if channels <= 2 {
            Mode::I | Mode::HISTOGRAM | Mode::TRUE_PEAK
        } else {
            Mode::TRUE_PEAK
        };
        let mut ebu = EbuR128::new(u32::from(channels), rate, mode)
            .map_err(|e| format!("cannot initialize loudness analysis: {e}"))?;
        if channels == 1 {
            ebu.set_channel(0, Channel::Center)
                .map_err(|e| e.to_string())?;
        }
        Ok(Self {
            channels: (0..channels).map(|_| ChannelMeter::default()).collect(),
            ebu,
            positive_full_scale,
            samples: 0,
            clipped_frames: 0,
            frame_clipped: false,
            left: 0.0,
            stereo: (channels == 2).then(StereoMeter::default),
            block: Vec::with_capacity(BLOCK_FRAMES * usize::from(channels)),
        })
    }

    fn consume(
        &mut self,
        samples: impl Iterator<Item = Result<f64, hound::Error>>,
    ) -> Result<(), String> {
        let channels = self.channels.len();
        for sample in samples {
            let value = sample.map_err(|e| format!("cannot decode WAV sample: {e}"))?;
            if !value.is_finite() {
                return Err("WAV contains a non-finite floating-point sample".into());
            }
            let channel = self.samples as usize % channels;
            let meter = &mut self.channels[channel];
            meter.peak = meter.peak.max(value.abs());
            meter.square_sum += value * value;
            meter.samples += 1;
            if value >= self.positive_full_scale || value <= -1.0 {
                meter.clipped += 1;
                self.frame_clipped = true;
            }
            if channel == 0 {
                self.left = value;
            } else if channel == 1
                && let Some(stereo) = &mut self.stereo
            {
                stereo.add(self.left, value);
            }
            if channel + 1 == channels {
                self.clipped_frames += u64::from(self.frame_clipped);
                self.frame_clipped = false;
            }
            self.samples += 1;
            self.block.push(value);
            if self.block.len() == BLOCK_FRAMES * channels {
                self.flush()?;
            }
        }
        if !(self.samples as usize).is_multiple_of(channels) {
            return Err("decoded audio ends inside an interleaved frame".into());
        }
        self.flush()
    }

    fn flush(&mut self) -> Result<(), String> {
        if !self.block.is_empty() {
            self.ebu
                .add_frames_f64(&self.block)
                .map_err(|e| format!("loudness analysis failed: {e}"))?;
            self.block.clear();
        }
        Ok(())
    }

    fn frames(&self) -> u64 {
        self.samples / self.channels.len() as u64
    }

    fn finish(mut self, spec: hound::WavSpec, file_bytes: u64) -> Result<AudioAnalysis, String> {
        let frames = self.frames();
        let peak = self.channels.iter().map(|c| c.peak).fold(0.0, f64::max);
        let square_sum: f64 = self.channels.iter().map(|c| c.square_sum).sum();
        let global_rms = rms(square_sum, self.samples);
        let integrated_lufs = if spec.channels <= 2 {
            let loudness = self.ebu.loudness_global().map_err(|e| e.to_string())?;
            if loudness.is_nan() || loudness == f64::INFINITY {
                return Err("loudness analysis produced a non-finite result".into());
            }
            finite(loudness)
        } else {
            None
        };
        // Flush the FIR tail only after measuring programme loudness and duration.
        self.ebu
            .add_frames_f64(&vec![0.0; 64 * usize::from(spec.channels)])
            .map_err(|e| format!("true-peak tail analysis failed: {e}"))?;
        let mut channels = Vec::with_capacity(self.channels.len());
        let mut global_true_peak: f64 = 0.0;
        for (index, channel) in self.channels.into_iter().enumerate() {
            let true_peak = self
                .ebu
                .true_peak(index as u32)
                .map_err(|e| e.to_string())?;
            if !true_peak.is_finite() || true_peak < 0.0 {
                return Err("true-peak analysis overflowed or produced an invalid result".into());
            }
            global_true_peak = global_true_peak.max(true_peak);
            let channel_rms = rms(channel.square_sum, channel.samples);
            channels.push(ChannelAnalysis {
                index: index as u16,
                samples: channel.samples,
                sample_peak_linear: channel.peak,
                sample_peak_dbfs: db(channel.peak),
                true_peak_dbtp: db(true_peak),
                rms_linear: channel_rms,
                rms_dbfs: db(channel_rms),
                crest_db: crest(channel.peak, channel_rms),
                samples_at_or_above_full_scale: channel.clipped,
            });
        }
        let mut limitations = vec![
            "WAV only; PCM 8/16/24/32-bit or float32; no compressed formats, RF64 or big-endian RIFX",
            "Input limits are 2 GiB, four hours, 32 channels, 8-384 kHz and 1 MiB of metadata before the audio data",
            "LUFS uses the ebur128 histogram implementation; this module has synthetic checks, not an independent full EBU conformance run",
            "A full-scale sample is an indicator; sample counts do not prove audible clipping distortion",
            "Null dB values represent silence or an undefined ratio; no NaN or infinity is serialized",
        ];
        if spec.channels > 2 {
            limitations.push("Integrated LUFS is unavailable for multichannel input because channel layout is not interpreted; every channel is retained for peak and RMS analysis");
        }
        if spec.channels != 2 {
            limitations.push("Stereo metrics require exactly two channels; mono is not duplicated and multichannel audio is not reduced to its first two channels");
        }
        Ok(AudioAnalysis {
            source: "file",
            format: "wav",
            status: if frames == 0 {
                SignalStatus::Empty
            } else if peak == 0.0 {
                SignalStatus::Silence
            } else {
                SignalStatus::Measured
            },
            file_bytes,
            sample_rate_hz: spec.sample_rate,
            channel_count: spec.channels,
            bits_per_sample: spec.bits_per_sample,
            sample_format: if spec.sample_format == SampleFormat::Int {
                "integer_pcm"
            } else {
                "ieee_float"
            },
            frames,
            samples: self.samples,
            duration_seconds: frames as f64 / f64::from(spec.sample_rate),
            sample_peak_linear: peak,
            sample_peak_dbfs: db(peak),
            true_peak_dbtp: db(global_true_peak),
            rms_linear: global_rms,
            rms_dbfs: db(global_rms),
            crest_db: crest(peak, global_rms),
            integrated_lufs,
            loudness_status: if spec.channels > 2 {
                "channel_layout_unknown"
            } else if frames == 0 {
                "empty"
            } else if peak == 0.0 {
                "silence"
            } else if integrated_lufs.is_none() {
                "below_gate_or_shorter_than_integration_window"
            } else {
                "measured"
            },
            clipping: ClippingAnalysis {
                samples_at_or_above_full_scale: channels
                    .iter()
                    .map(|c| c.samples_at_or_above_full_scale)
                    .sum(),
                frames_at_or_above_full_scale: self.clipped_frames,
                definition: if spec.sample_format == SampleFormat::Int {
                    "integer PCM samples at minimum or maximum representable code; frames count once if any channel meets that condition"
                } else {
                    "float samples whose absolute value is at least 1.0; frames count once if any channel meets that condition"
                },
            },
            channels,
            stereo: self.stereo.map(StereoMeter::finish),
            method: AnalysisMethod {
                decoder: "hound streaming WAV decoder",
                loudness: "ebur128 integrated mode with bounded histogram history",
                true_peak: "ebur128 FIR oversampling; 4x below 96 kHz, 2x below 192 kHz, sample peak at 192 kHz and above",
                loudness_channel_layout: if spec.channels == 1 {
                    "mono center, counted once"
                } else if spec.channels == 2 {
                    "left/right stereo"
                } else {
                    "unknown; integrated LUFS omitted"
                },
                true_peak_tail_padding_frames: 64,
                rms_definition: "square-root mean sample energy; global RMS averages all samples across all channels",
            },
            limitations,
        })
    }
}

fn finite(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

fn db(value: f64) -> Option<f64> {
    if value > 0.0 {
        finite(20.0 * value.log10())
    } else {
        None
    }
}

fn rms(square_sum: f64, samples: u64) -> f64 {
    if samples == 0 {
        0.0
    } else {
        (square_sum / samples as f64).sqrt()
    }
}

fn crest(peak: f64, rms: f64) -> Option<f64> {
    if rms > 0.0 { db(peak / rms) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    fn float_wav(
        channels: u16,
        frames: u32,
        sample: impl Fn(u32, u16) -> f32,
    ) -> tempfile::NamedTempFile {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut writer = hound::WavWriter::create(
            file.path(),
            hound::WavSpec {
                channels,
                sample_rate: 48_000,
                bits_per_sample: 32,
                sample_format: SampleFormat::Float,
            },
        )
        .unwrap();
        for frame in 0..frames {
            for channel in 0..channels {
                writer.write_sample(sample(frame, channel)).unwrap();
            }
        }
        writer.finalize().unwrap();
        file
    }

    #[test]
    fn silence_and_empty_audio_use_finite_json_and_distinct_statuses() {
        let silence = float_wav(1, 48_000, |_, _| 0.0);
        let report = analyze_file(silence.path()).unwrap();
        assert_eq!(report.status, SignalStatus::Silence);
        assert_eq!(report.frames, 48_000);
        assert_eq!(report.samples, 48_000);
        assert_eq!(report.integrated_lufs, None);
        assert_eq!(report.sample_peak_dbfs, None);
        assert_eq!(report.true_peak_dbtp, None);
        assert!(report.stereo.is_none());
        assert_eq!(
            serde_json::to_value(&report).unwrap()["rms_dbfs"],
            serde_json::Value::Null
        );
        let empty = float_wav(2, 0, |_, _| 0.0);
        let report = analyze_file(empty.path()).unwrap();
        assert_eq!(report.status, SignalStatus::Empty);
        assert_eq!(report.duration_seconds, 0.0);
    }

    #[test]
    fn calibrated_stereo_sine_reports_peak_rms_loudness_and_mono_correlation() {
        let file = float_wav(2, 96_000, |frame, _| {
            (0.1 * (TAU * 1000.0 * f64::from(frame) / 48_000.0).sin()) as f32
        });
        let report = analyze_file(file.path()).unwrap();
        assert_eq!(report.frames, 96_000);
        assert_eq!(report.samples, 192_000);
        assert!((report.sample_peak_dbfs.unwrap() + 20.0).abs() < 0.0001);
        assert!((report.rms_dbfs.unwrap() + 23.0103).abs() < 0.0001);
        assert!((report.crest_db.unwrap() - 3.0103).abs() < 0.0001);
        assert!((report.integrated_lufs.unwrap() + 20.0).abs() < 0.25);
        assert!(report.true_peak_dbtp.unwrap() >= report.sample_peak_dbfs.unwrap() - 0.0001);
        let stereo = report.stereo.unwrap();
        assert!((stereo.correlation.unwrap() - 1.0).abs() < 1e-9);
        assert_eq!(stereo.side_energy_fraction, Some(0.0));
        assert_eq!(report.clipping.samples_at_or_above_full_scale, 0);
        let mono = float_wav(1, 96_000, |frame, _| {
            (0.1 * (TAU * 1000.0 * f64::from(frame) / 48_000.0).sin()) as f32
        });
        let mono_report = analyze_file(mono.path()).unwrap();
        assert!((mono_report.integrated_lufs.unwrap() + 23.0103).abs() < 0.25);
        assert!(mono_report.stereo.is_none());
    }

    #[test]
    fn antiphase_stereo_keeps_both_channels_and_reports_silent_mid() {
        let file = float_wav(2, 48_000, |frame, channel| {
            let signal = (0.2 * (TAU * 1000.0 * f64::from(frame) / 48_000.0).sin()) as f32;
            if channel == 0 { signal } else { -signal }
        });
        let report = analyze_file(file.path()).unwrap();
        let stereo = report.stereo.unwrap();
        assert!((stereo.correlation.unwrap() + 1.0).abs() < 1e-9);
        assert_eq!(stereo.mid_rms_linear, 0.0);
        assert_eq!(stereo.side_to_mid_db, None);
        assert_eq!(stereo.side_to_mid_status, "mid_silent");
        assert_eq!(stereo.side_energy_fraction, Some(1.0));
    }

    #[test]
    fn multichannel_peak_and_clipping_include_the_fourth_channel() {
        let file = float_wav(4, 128, |frame, channel| {
            if frame == 0 && channel == 3 {
                1.25
            } else {
                0.0
            }
        });
        let report = analyze_file(file.path()).unwrap();
        assert_eq!(report.channels.len(), 4);
        assert_eq!(report.channels[0].sample_peak_linear, 0.0);
        assert_eq!(report.channels[3].sample_peak_linear, 1.25);
        assert_eq!(report.sample_peak_linear, 1.25);
        assert_eq!(report.clipping.samples_at_or_above_full_scale, 1);
        assert_eq!(report.clipping.frames_at_or_above_full_scale, 1);
        assert_eq!(report.integrated_lufs, None);
        assert_eq!(report.loudness_status, "channel_layout_unknown");
        assert!(report.stereo.is_none());
    }

    #[test]
    fn pcm24_normalization_distinguishes_full_scale_samples_and_frames() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut writer = hound::WavWriter::create(
            file.path(),
            hound::WavSpec {
                channels: 2,
                sample_rate: 48_000,
                bits_per_sample: 24,
                sample_format: SampleFormat::Int,
            },
        )
        .unwrap();
        for sample in [8_388_607_i32, -8_388_608, 4_194_304, 0] {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
        let report = analyze_file(file.path()).unwrap();
        assert_eq!(report.frames, 2);
        assert_eq!(report.samples, 4);
        assert_eq!(report.sample_peak_linear, 1.0);
        assert_eq!(report.clipping.samples_at_or_above_full_scale, 2);
        assert_eq!(report.clipping.frames_at_or_above_full_scale, 1);
        assert_eq!(report.channels[0].samples_at_or_above_full_scale, 1);
        assert_eq!(report.channels[1].samples_at_or_above_full_scale, 1);
        assert_eq!(report.integrated_lufs, None);
    }

    #[test]
    fn pcm8_odd_sized_audio_data_is_read_without_duplicating_samples() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut writer = hound::WavWriter::create(
            file.path(),
            hound::WavSpec {
                channels: 1,
                sample_rate: 48_000,
                bits_per_sample: 8,
                sample_format: SampleFormat::Int,
            },
        )
        .unwrap();
        for sample in [0_i8, 64, 127] {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
        let report = analyze_file(file.path()).unwrap();
        assert_eq!(report.frames, 3);
        assert_eq!(report.samples, 3);
        assert_eq!(report.sample_peak_linear, 127.0 / 128.0);
        assert_eq!(report.clipping.samples_at_or_above_full_scale, 1);
    }

    #[test]
    fn invalid_nonfinite_samples_and_excessive_channel_counts_are_rejected() {
        let file = float_wav(1, 8, |frame, _| if frame == 3 { f32::NAN } else { 0.0 });
        assert!(
            analyze_file(file.path())
                .unwrap_err()
                .contains("non-finite")
        );
        let excessive = float_wav(33, 1, |_, _| 0.0);
        assert!(
            analyze_file(excessive.path())
                .unwrap_err()
                .contains("channels")
        );
    }

    #[test]
    fn truncated_audio_and_oversized_metadata_reject_before_analysis() {
        let file = float_wav(1, 8, |_, _| 0.0);
        file.as_file()
            .set_len(file.as_file().metadata().unwrap().len() - 1)
            .unwrap();
        assert!(analyze_file(file.path()).is_err());
        let malformed = tempfile::NamedTempFile::new().unwrap();
        let mut bytes = b"RIFF".to_vec();
        bytes.extend_from_slice(&(1_048_600_u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVEJUNK");
        bytes.extend_from_slice(&(1_048_580_u32).to_le_bytes());
        bytes.resize(1_048_608, 0);
        std::fs::write(malformed.path(), bytes).unwrap();
        assert!(
            analyze_file(malformed.path())
                .unwrap_err()
                .contains("metadata")
        );
    }
}
