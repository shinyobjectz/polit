use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

/// Commands sent from any screen to the audio thread.
enum MusicCommand {
    PlayNavSfx,
    PlaySelectSfx,
    PlayTypewriterTick,
    SwitchToIntro,
    AdvanceSlide(usize),
    SwitchToAnthem,
    Shutdown,
}

/// Shared music controller — lives for the entire app session.
/// Owns a channel to a dedicated audio thread that plays all sounds.
pub struct MusicController {
    cmd_tx: mpsc::Sender<MusicCommand>,
    muted: Arc<AtomicBool>,
    _handle: Option<std::thread::JoinHandle<()>>,
}

impl MusicController {
    /// Spawn the audio thread and start the title anthem.
    pub fn start_anthem() -> Self {
        let muted = Arc::new(AtomicBool::new(false));
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let muted_ref = muted.clone();

        let handle = std::thread::Builder::new()
            .name("polit-music".to_string())
            .spawn(move || {
                if let Err(e) = run_audio_thread(cmd_rx, muted_ref) {
                    eprintln!("[music] audio thread error: {e}");
                }
            })
            .expect("Failed to spawn music thread");

        Self {
            cmd_tx,
            muted,
            _handle: Some(handle),
        }
    }

    pub fn toggle_mute(&self) -> bool {
        let was = self.muted.load(Ordering::Relaxed);
        self.muted.store(!was, Ordering::Relaxed);
        !was
    }

    pub fn is_muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed)
    }

    /// Menu navigation tick (arrow keys).
    pub fn play_nav(&self) {
        let _ = self.cmd_tx.send(MusicCommand::PlayNavSfx);
    }

    /// Menu selection confirm (enter key).
    pub fn play_select(&self) {
        let _ = self.cmd_tx.send(MusicCommand::PlaySelectSfx);
    }

    /// Typewriter character tick for intro cards.
    pub fn play_typewriter_tick(&self) {
        let _ = self.cmd_tx.send(MusicCommand::PlayTypewriterTick);
    }

    /// Cross-fade from anthem to the intro cinematic score.
    pub fn switch_to_intro(&self) {
        let _ = self.cmd_tx.send(MusicCommand::SwitchToIntro);
    }

    /// Advance the intro score to a new slide.
    pub fn advance_slide(&self, index: usize) {
        let _ = self.cmd_tx.send(MusicCommand::AdvanceSlide(index));
    }

    /// Switch back to the title anthem loop.
    pub fn switch_to_anthem(&self) {
        let _ = self.cmd_tx.send(MusicCommand::SwitchToAnthem);
    }

    /// Shut down the audio thread completely.
    pub fn stop(&self) {
        let _ = self.cmd_tx.send(MusicCommand::Shutdown);
    }
}

impl Drop for MusicController {
    fn drop(&mut self) {
        self.stop();
    }
}

// ── Frequencies (Bb major) ──────────────────────────────────────────

const EB3: f32 = 155.56;
const F3: f32 = 174.61;
const G3: f32 = 196.00;
const AB3: f32 = 207.65;
const BB3: f32 = 233.08;
const C4: f32 = 261.63;
const D4: f32 = 293.66;
const EB4: f32 = 311.13;
const F4: f32 = 349.23;
const BB4: f32 = 466.16;
const D5: f32 = 587.33;
const F5: f32 = 698.46;
const BB5: f32 = 932.33;

// ── Compositions ────────────────────────────────────────────────────

fn compose_anthem() -> tunes::composition::Composition {
    use tunes::prelude::*;

    let mut comp = Composition::new(Tempo::new(30.0));

    // Pad layer: sustained Bb-major harmonic bed
    comp.track("pad")
        .reverb(Reverb::new(0.95, 0.15, 0.85))
        .filter(Filter::low_pass(600.0, 0.2))
        .volume(0.18)
        .note(&[BB3, D4, F4], 16.0)
        .note(&[EB3, G3, BB3], 16.0)
        .note(&[F3, AB3, C4], 16.0)
        .note(&[BB3, D4, F4], 16.0);

    // Melody: Star-Spangled Banner opening, very slow
    comp.track("melody")
        .reverb(Reverb::new(0.9, 0.2, 0.75))
        .filter(Filter::low_pass(1000.0, 0.3))
        .volume(0.15)
        .note(&[BB3], 3.0)
        .note(&[G3], 2.0)
        .note(&[EB3], 3.0)
        .note(&[G3], 2.0)
        .note(&[BB3], 3.0)
        .note(&[BB3], 2.0)
        .note(&[D4], 2.0)
        .note(&[D5], 4.0)
        .note(&[C4], 2.5)
        .note(&[BB3], 2.5)
        .note(&[BB4], 6.0)
        .wait(2.0)
        .note(&[BB3], 2.0)
        .note(&[BB3], 1.5)
        .note(&[D4], 2.5)
        .note(&[F4], 2.0)
        .note(&[F4], 1.5)
        .note(&[EB4], 2.0)
        .note(&[D4], 2.0)
        .note(&[BB3], 6.0);

    // Sub-bass
    comp.track("bass")
        .reverb(Reverb::new(0.8, 0.4, 0.5))
        .filter(Filter::low_pass(200.0, 0.1))
        .volume(0.08)
        .note(&[BB3 / 2.0], 16.0)
        .note(&[EB3 / 2.0], 16.0)
        .note(&[F3 / 2.0], 16.0)
        .note(&[BB3 / 2.0], 16.0);

    comp
}

/// Short Bb blip for arrow key navigation.
fn compose_nav_sfx() -> tunes::composition::Composition {
    use tunes::prelude::*;
    let mut comp = Composition::new(Tempo::new(240.0));
    comp.track("tick")
        .filter(Filter::low_pass(2000.0, 0.3))
        .volume(0.12)
        .note(&[F5], 0.04);
    comp
}

/// Bb major triad for menu selection confirm.
fn compose_select_sfx() -> tunes::composition::Composition {
    use tunes::prelude::*;
    let mut comp = Composition::new(Tempo::new(120.0));
    comp.track("confirm")
        .reverb(Reverb::new(0.4, 0.3, 0.35))
        .volume(0.18)
        .note(&[BB4, D5, F5], 0.15);
    comp
}

/// Tiny percussive tick for typewriter effect.
fn compose_typewriter_tick() -> tunes::composition::Composition {
    use tunes::prelude::*;
    let mut comp = Composition::new(Tempo::new(480.0));
    comp.track("tick").volume(0.05).note(&[BB5], 0.012);
    comp
}

/// Intro slide 0: "The year is 2024." — sparse, a single sustained note.
fn compose_intro_slide_0() -> tunes::composition::Composition {
    use tunes::prelude::*;
    let mut comp = Composition::new(Tempo::new(25.0));
    comp.track("pad")
        .reverb(Reverb::new(0.95, 0.15, 0.9))
        .filter(Filter::low_pass(500.0, 0.2))
        .volume(0.12)
        .note(&[BB3], 24.0);
    comp
}

/// Intro slide 1: "A nation at a crossroads." — adds texture, Bb→Eb.
fn compose_intro_slide_1() -> tunes::composition::Composition {
    use tunes::prelude::*;
    let mut comp = Composition::new(Tempo::new(25.0));
    comp.track("pad")
        .reverb(Reverb::new(0.95, 0.15, 0.9))
        .filter(Filter::low_pass(600.0, 0.2))
        .volume(0.14)
        .note(&[BB3, D4, F4], 12.0)
        .note(&[EB3, G3, BB3], 12.0);
    comp.track("high")
        .reverb(Reverb::new(0.9, 0.2, 0.8))
        .filter(Filter::low_pass(1200.0, 0.3))
        .volume(0.06)
        .wait(6.0)
        .note(&[F4], 8.0)
        .note(&[EB4], 10.0);
    comp
}

/// Intro slide 2: "But politics isn't just about..." — warmer, F→Bb.
fn compose_intro_slide_2() -> tunes::composition::Composition {
    use tunes::prelude::*;
    let mut comp = Composition::new(Tempo::new(28.0));
    comp.track("pad")
        .reverb(Reverb::new(0.95, 0.15, 0.85))
        .filter(Filter::low_pass(700.0, 0.2))
        .volume(0.15)
        .note(&[F3, AB3, C4], 10.0)
        .note(&[BB3, D4, F4], 10.0);
    comp.track("melody")
        .reverb(Reverb::new(0.9, 0.2, 0.75))
        .filter(Filter::low_pass(1000.0, 0.3))
        .volume(0.08)
        .wait(3.0)
        .note(&[D4], 5.0)
        .note(&[BB3], 5.0)
        .note(&[C4], 7.0);
    comp
}

/// Intro slide 3: "This is your story." — resolution, full Bb major.
fn compose_intro_slide_3() -> tunes::composition::Composition {
    use tunes::prelude::*;
    let mut comp = Composition::new(Tempo::new(30.0));
    comp.track("pad")
        .reverb(Reverb::new(0.95, 0.1, 0.85))
        .filter(Filter::low_pass(800.0, 0.25))
        .volume(0.16)
        .note(&[BB3, D4, F4, BB4], 24.0);
    comp.track("melody")
        .reverb(Reverb::new(0.9, 0.15, 0.8))
        .filter(Filter::low_pass(1200.0, 0.3))
        .volume(0.10)
        .wait(4.0)
        .note(&[F4], 4.0)
        .note(&[D5], 6.0)
        .note(&[BB4], 10.0);
    comp
}

// ── Audio thread ────────────────────────────────────────────────────

fn run_audio_thread(
    cmd_rx: mpsc::Receiver<MusicCommand>,
    muted: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    use tunes::prelude::*;

    let engine = AudioEngine::new()?;

    // Pre-compose everything
    let anthem_mixer = compose_anthem().into_mixer();
    let nav_sfx = compose_nav_sfx().into_mixer();
    let select_sfx = compose_select_sfx().into_mixer();
    let tick_sfx = compose_typewriter_tick().into_mixer();
    let intro_slides: Vec<_> = vec![
        compose_intro_slide_0().into_mixer(),
        compose_intro_slide_1().into_mixer(),
        compose_intro_slide_2().into_mixer(),
        compose_intro_slide_3().into_mixer(),
    ];

    // Start with the anthem
    let mut current_loop: Option<SoundId> = None;
    match engine.play_looping(&anthem_mixer) {
        Ok(id) => {
            engine.set_volume(id, 0.5).ok();
            current_loop = Some(id);
        }
        Err(e) => eprintln!("[music] anthem start failed: {e}"),
    }

    loop {
        match cmd_rx.try_recv() {
            Ok(MusicCommand::Shutdown) => {
                engine.stop_all().ok();
                break;
            }
            Ok(MusicCommand::PlayNavSfx) => {
                if !muted.load(Ordering::Relaxed) {
                    engine.play_mixer(&nav_sfx).ok();
                }
            }
            Ok(MusicCommand::PlaySelectSfx) => {
                if !muted.load(Ordering::Relaxed) {
                    engine.play_mixer(&select_sfx).ok();
                }
            }
            Ok(MusicCommand::PlayTypewriterTick) => {
                if !muted.load(Ordering::Relaxed) {
                    engine.play_mixer(&tick_sfx).ok();
                }
            }
            Ok(MusicCommand::SwitchToIntro) => {
                if let Some(id) = current_loop.take() {
                    engine.stop(id).ok();
                }
                if let Some(mixer) = intro_slides.first() {
                    if let Ok(id) = engine.play_looping(mixer) {
                        let vol = if muted.load(Ordering::Relaxed) {
                            0.0
                        } else {
                            0.4
                        };
                        engine.set_volume(id, vol).ok();
                        current_loop = Some(id);
                    }
                }
            }
            Ok(MusicCommand::AdvanceSlide(idx)) => {
                if let Some(id) = current_loop.take() {
                    engine.stop(id).ok();
                }
                if let Some(mixer) = intro_slides.get(idx) {
                    if let Ok(id) = engine.play_looping(mixer) {
                        let vol = if muted.load(Ordering::Relaxed) {
                            0.0
                        } else {
                            0.4
                        };
                        engine.set_volume(id, vol).ok();
                        current_loop = Some(id);
                    }
                }
            }
            Ok(MusicCommand::SwitchToAnthem) => {
                if let Some(id) = current_loop.take() {
                    engine.stop(id).ok();
                }
                if let Ok(id) = engine.play_looping(&anthem_mixer) {
                    let vol = if muted.load(Ordering::Relaxed) {
                        0.0
                    } else {
                        0.5
                    };
                    engine.set_volume(id, vol).ok();
                    current_loop = Some(id);
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {}
        }

        // Keep volume in sync with mute state
        if let Some(id) = current_loop {
            let vol = if muted.load(Ordering::Relaxed) {
                0.0
            } else {
                0.5
            };
            engine.set_volume(id, vol).ok();
        }

        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    Ok(())
}
