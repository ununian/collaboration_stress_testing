use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref ID_GENERATOR: Mutex<IdGenerator> = Mutex::new(IdGenerator::new());
}

struct IdGenerator {
    current: u64,
}

impl IdGenerator {
    fn new() -> Self {
        IdGenerator { current: 0 }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.current;
        self.current += 1;
        id
    }

    fn reset(&mut self) {
        self.current = 0;
    }
}

#[cfg(test)]
pub fn next_id() -> String {
    let mut generator = ID_GENERATOR.lock().unwrap();
    generator.next_id().to_string()
}

#[cfg(not(test))]
pub fn next_id() -> String {
    nanoid::nanoid!()
}

fn reset_id_generator() {
    let mut generator = ID_GENERATOR.lock().unwrap();
    generator.reset()
}
