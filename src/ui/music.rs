use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Background music controller for the title screen.
/// Spawns a dedicated audio thread and communicates via atomics.
pub struct MusicController {
    muted: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    _handle: Option<std::thread::JoinHandle<()>>,
}

impl MusicController {
    /// Start ambient anthem playback on a background thread.
    pub fn start_anthem() -> Self {
        let muted = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));

        let muted_ref = muted.clone();
        let stop_ref = stop.clone();

        let handle = std::thread::Builder::new()
            .name("polit-music".to_string())
            .spawn(move || {
                if let Err(e) = run_music_thread(muted_ref, stop_ref) {
                    eprintln!("[music] playback error: {e}");
                }
            })
            .expect("Failed to spawn music thread");

        Self {
            muted,
            stop,
            _handle: Some(handle),
        }
    }

    /// Toggle mute state. Returns true if now muted.
    pub fn toggle_mute(&self) -> bool {
        let was_muted = self.muted.load(Ordering::Relaxed);
        self.muted.store(!was_muted, Ordering::Relaxed);
        !was_muted
    }

    pub fn is_muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Drop for MusicController {
    fn drop(&mut self) {
        self.stop();
    }
}

// ── Star-Spangled Banner frequencies (Bb major) ─────────────────────

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

/// Compose a slowed, reverbed ambient arrangement of the
/// Star-Spangled Banner opening in Bb major.
fn compose_anthem() -> tunes::composition::Composition {
    use tunes::prelude::*;

    // Very slow — dreamy, ambient tempo
    let mut comp = Composition::new(Tempo::new(30.0));

    // ── Pad layer: sustained Bb-major harmonic bed ──────────────
    comp.track("pad")
        .reverb(Reverb::new(0.95, 0.15, 0.85))
        .filter(Filter::low_pass(600.0, 0.2))
        .volume(0.18)
        // Bb major
        .note(&[BB3, D4, F4], 16.0)
        // Eb major
        .note(&[EB3, G3, BB3], 16.0)
        // F major (dominant)
        .note(&[F3, AB3, C4], 16.0)
        // Bb major resolve
        .note(&[BB3, D4, F4], 16.0);

    // ── Melody: opening phrase, very slow and soft ──────────────
    // "Oh say can you see, by the dawn's early light"
    comp.track("melody")
        .reverb(Reverb::new(0.9, 0.2, 0.75))
        .filter(Filter::low_pass(1000.0, 0.3))
        .volume(0.15)
        // "Oh say"
        .note(&[BB3], 3.0)
        .note(&[G3], 2.0)
        // "can you see"
        .note(&[EB3], 3.0)
        .note(&[G3], 2.0)
        .note(&[BB3], 3.0)
        // "by the"
        .note(&[BB3], 2.0)
        .note(&[D4], 2.0)
        // "dawn's"
        .note(&[D5], 4.0)
        // "early"
        .note(&[C4], 2.5)
        .note(&[BB3], 2.5)
        // "light" (held)
        .note(&[BB4], 6.0)
        // "What so"
        .wait(2.0)
        .note(&[BB3], 2.0)
        .note(&[BB3], 1.5)
        // "proudly we hailed"
        .note(&[D4], 2.5)
        .note(&[F4], 2.0)
        .note(&[F4], 1.5)
        .note(&[EB4], 2.0)
        .note(&[D4], 2.0)
        // resolve
        .note(&[BB3], 6.0);

    // ── Sub-bass: root notes, very low and warm ─────────────────
    comp.track("bass")
        .reverb(Reverb::new(0.8, 0.4, 0.5))
        .filter(Filter::low_pass(200.0, 0.1))
        .volume(0.08)
        .note(&[BB3 / 2.0], 16.0) // Bb2
        .note(&[EB3 / 2.0], 16.0) // Eb2
        .note(&[F3 / 2.0], 16.0) // F2
        .note(&[BB3 / 2.0], 16.0); // Bb2

    comp
}

/// Audio thread main loop: compose, play, loop until stopped.
fn run_music_thread(
    muted: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    use tunes::prelude::*;

    let engine = AudioEngine::new()?;
    let comp = compose_anthem();
    let mixer = comp.into_mixer();

    // Start looping playback — returns a SoundId for volume control
    let id = engine.play_looping(&mixer)?;
    engine.set_volume(id, 0.5)?;

    // Poll for mute/stop
    loop {
        if stop.load(Ordering::Relaxed) {
            engine.stop_all()?;
            break;
        }

        let vol = if muted.load(Ordering::Relaxed) {
            0.0
        } else {
            0.5
        };
        engine.set_volume(id, vol)?;

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Ok(())
}
