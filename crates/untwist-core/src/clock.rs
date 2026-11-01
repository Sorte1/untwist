#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant as StageInstant;

#[cfg(target_arch = "wasm32")]
pub use stopped::Instant;

#[cfg(target_arch = "wasm32")]
pub use stage::{set_stage_clock, StageInstant};

#[cfg(target_arch = "wasm32")]
mod stopped {
    use std::time::Duration;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Instant;

    impl Instant {
        pub fn now() -> Self {
            Instant
        }

        pub fn elapsed(&self) -> Duration {
            Duration::ZERO
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod stage {
    use std::sync::OnceLock;
    use std::time::Duration;

    static CLOCK: OnceLock<fn() -> f64> = OnceLock::new();

    pub fn set_stage_clock(clock: fn() -> f64) -> bool {
        CLOCK.set(clock).is_ok()
    }

    fn reading() -> f64 {
        CLOCK.get().map_or(0.0, |clock| clock())
    }

    #[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
    pub struct StageInstant(f64);

    impl StageInstant {
        pub fn now() -> Self {
            Self(reading())
        }

        pub fn elapsed(&self) -> Duration {
            let ms = reading() - self.0;
            if ms.is_finite() && ms > 0.0 {
                Duration::from_secs_f64(ms / 1000.0)
            } else {
                Duration::ZERO
            }
        }
    }
}
