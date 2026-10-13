use std::sync::mpsc::{self, Receiver, Sender};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Media::Speech::*,
        System::Com::*,
        UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK},
    },
};
pub enum Command {
    Speak(String),
    Stop,
}
pub struct Speech {
    pub send: Sender<Command>,
    pub errors: Receiver<String>,
}
unsafe fn local_voice() -> windows::core::Result<ISpVoice> {
    let voice: ISpVoice = CoCreateInstance(&SpVoice, None, CLSCTX_INPROC_SERVER)?;
    let category: ISpObjectTokenCategory =
        CoCreateInstance(&SpObjectTokenCategory, None, CLSCTX_INPROC_SERVER)?;
    category.SetId(SPCAT_VOICES, false)?;
    let tokens = category.EnumTokens(w!("Vendor=Microsoft"), PCWSTR::null())?;
    voice.SetVoice(&tokens.Item(0)?)?;
    Ok(voice)
}
impl Speech {
    pub fn new() -> Self {
        let (send, rx) = mpsc::channel();
        let (tx, errors) = mpsc::channel();
        std::thread::spawn(move || unsafe {
            if let Err(e) = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() {
                let _ = tx.send(e.to_string());
                return;
            }
            let result = (|| -> windows::core::Result<()> {
                // Only a locally installed Microsoft SAPI voice; no external speech provider.
                let voice = local_voice()?;
                for command in rx {
                    let result = match command {
                        Command::Speak(text) => {
                            let text: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
                            voice.Speak(
                                PCWSTR(text.as_ptr()),
                                (SPF_ASYNC.0 | SPF_PURGEBEFORESPEAK.0 | SPF_IS_NOT_XML.0) as u32,
                                None,
                            )
                        }
                        Command::Stop => {
                            voice.Speak(PCWSTR::null(), SPF_PURGEBEFORESPEAK.0 as u32, None)
                        }
                    };
                    if let Err(e) = result {
                        let _ = tx.send(e.to_string());
                    }
                }
                Ok(())
            })();
            if let Err(e) = result {
                let _=tx.send(format!("Local read-aloud is unavailable. Install a Microsoft Windows speech voice in Windows Settings. {e}"));
            }
            CoUninitialize();
        });
        Self { send, errors }
    }
}

pub fn error_dialog(text: &str) {
    let text: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            w!("Whispering"),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires a locally installed Microsoft Windows voice"]
    fn microsoft_voice_synthesizes_without_a_speaker_or_network() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("speech.wav");
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
            {
                let voice = local_voice().unwrap();
                let stream: ISpeechFileStream =
                    CoCreateInstance(&SpFileStream, None, CLSCTX_INPROC_SERVER).unwrap();
                stream
                    .Format()
                    .unwrap()
                    .SetType(SAFT22kHz16BitMono)
                    .unwrap();
                stream
                    .Open(
                        &windows::core::BSTR::from(path.to_string_lossy().as_ref()),
                        SSFMCreateForWrite,
                        windows::Win32::Foundation::VARIANT_BOOL(0),
                    )
                    .unwrap();
                voice.SetOutput(&stream, false).unwrap();
                voice
                    .Speak(
                        w!("Whispering keeps speech on this computer."),
                        SPF_IS_NOT_XML.0 as u32,
                        None,
                    )
                    .unwrap();
                stream.Close().unwrap();
            }
            CoUninitialize();
        }
        assert!(std::fs::metadata(&path).unwrap().len() > 1000);
        assert!(!crate::audio::read_audio(&path).unwrap().is_empty());
    }
}
