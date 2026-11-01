use std::cell::Cell;
use std::sync::OnceLock;

thread_local! {
    static INSIDE_BOUNDARY: Cell<bool> = const { Cell::new(false) };
}

fn install_silencer() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if INSIDE_BOUNDARY.with(Cell::get) {
                return;
            }
            previous(info);
        }));
    });
}

pub(super) struct Boundary {
    was_inside: bool,
}

impl Boundary {
    pub(super) fn enter() -> Self {
        Self::set(true)
    }

    pub(super) fn lifted(&self) -> Self {
        Self::set(self.was_inside)
    }

    fn set(silenced: bool) -> Self {
        install_silencer();
        let was_inside = INSIDE_BOUNDARY.with(|inside| inside.replace(silenced));
        Self { was_inside }
    }
}

#[cfg(test)]
pub(super) fn is_silenced() -> bool {
    INSIDE_BOUNDARY.with(Cell::get)
}

impl Drop for Boundary {
    fn drop(&mut self) {
        let was_inside = self.was_inside;
        INSIDE_BOUNDARY.with(|inside| inside.set(was_inside));
    }
}
