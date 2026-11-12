#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoEncoderKind {
    AvcHardware,
    HevcHardware,
    Mpeg4,
    Software,
    Custom,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioFilterPlan {
    pub filter_complex: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FfmpegPlan {
    pub video_input_args: Vec<String>,
    pub audio_input_specs: Vec<Vec<String>>,
    pub output_args: Vec<String>,
    pub audio_filter: AudioFilterPlan,
}

pub fn select_bitrate_control(
    dynamic: bool,
    encoder: &str,
    encoder_list: [&str; 4],
    mpeg4: bool,
    custom_encoder: Option<&str>,
) -> &'static str {
    if !dynamic {
        return "-b:v";
    }
    if encoder == encoder_list[0] && !mpeg4 {
        "-cq"
    } else if encoder == encoder_list[1] || mpeg4 {
        "-q"
    } else if encoder.ends_with("_vaapi") {
        "-qp"
    } else if encoder == encoder_list[2] {
        "-qp_p"
    } else if custom_encoder == Some(encoder) {
        "-q"
    } else {
        "-crf"
    }
}

pub fn build_audio_filter(
    volume_music: f32,
    volume_sfx: f32,
    speed: f32,
    loudness_equalization: bool,
    force_limit: bool,
    limit_threshold: f32,
    ending_delay_millis: f64,
    hires: bool,
) -> AudioFilterPlan {
    let mut music = if loudness_equalization {
        "[2:a]loudnorm=I=-16:LRA=24:TP=-1,aresample=48000:resampler=swr".to_owned()
    } else {
        "[2:a]aresample=48000:resampler=swr".to_owned()
    };
    music.push_str(&format!(",volume={volume_music}"));
    if speed != 1.0 {
        music.push_str(&format!(",rubberband=tempo={speed}"));
    }
    music.push_str("[a2];");

    let mut sfx = format!("[1:a]volume={volume_sfx}");
    if force_limit {
        sfx.push_str(&format!(
            ",alimiter=limit={limit_threshold}:level=false:attack=0.1:release=1"
        ));
    }
    sfx.push_str("[a1];");

    let ending = format!(
        "[3:a]volume={volume_music},adelay={ending_delay_millis}|{ending_delay_millis}[a3];"
    );
    let mix = if hires {
        "[a1][a2][a3]amix=inputs=3:duration=first:normalize=0[a]".to_owned()
    } else {
        "[a1][a2][a3]amix=inputs=3:duration=first:normalize=0[aa];[aa]alimiter=limit=1.0:level=false:attack=0.1:release=1[a]".to_owned()
    };

    AudioFilterPlan {
        filter_complex: format!("{music}{sfx}{ending}{mix}"),
    }
}

pub fn build_output_args(
    encoder: &str,
    bitrate_control: &str,
    bitrate: &str,
    filter_complex: &str,
    hires: bool,
) -> Vec<String> {
    let (audio_codec, audio_bitrate, container) = if hires {
        ("pcm_f32le", None, "mov")
    } else {
        ("aac", Some("320k"), "mp4")
    };
    let mut args = vec![
        "-c:a".to_owned(),
        audio_codec.to_owned(),
        "-c:v".to_owned(),
        encoder.to_owned(),
        "-movflags".to_owned(),
        "+faststart".to_owned(),
        "-pix_fmt".to_owned(),
        "yuv420p".to_owned(),
        bitrate_control.to_owned(),
        bitrate.to_owned(),
    ];
    if let Some(audio_bitrate) = audio_bitrate {
        args.extend(["-b:a".to_owned(), audio_bitrate.to_owned()]);
    }
    args.extend([
        "-filter_complex".to_owned(),
        filter_complex.to_owned(),
        "-map".to_owned(),
        "0:v:0".to_owned(),
        "-map".to_owned(),
        "[a]".to_owned(),
        "-f".to_owned(),
        container.to_owned(),
    ]);
    args
}

pub fn build_video_input_args(
    width: u32,
    height: u32,
    fps: u32,
    encoder: &str,
    hardware_encoder: &str,
) -> Vec<String> {
    let mut args = vec![
        "-probesize".to_owned(),
        "50M".to_owned(),
        "-y".to_owned(),
        "-f".to_owned(),
        "rawvideo".to_owned(),
        "-c:v".to_owned(),
        "rawvideo".to_owned(),
        "-color_range".to_owned(),
        "full".to_owned(),
    ];
    if encoder == hardware_encoder {
        args.extend(["-hwaccel_output_format", "cuda"].map(str::to_owned));
    }
    args.extend(
        [
            "-s",
            &format!("{width}x{height}"),
            "-r",
            &fps.to_string(),
            "-pix_fmt",
            "yuv420p",
            "-thread_queue_size",
            "1024",
            "-i",
            "pipe:0",
        ]
        .map(str::to_owned),
    );
    args
}
