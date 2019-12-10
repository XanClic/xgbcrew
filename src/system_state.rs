pub struct SystemState {
    pub cgb: bool,
}


impl SystemState {
    pub fn new() -> Self {
        Self {
            cgb: false,
        }
    }
}
