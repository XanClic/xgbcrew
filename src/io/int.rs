#[repr(u8)]
#[allow(dead_code)]
pub enum Irq {
    VBlank  = 1 << 0,
    Lcdc    = 1 << 1,
    Timer   = 1 << 2,
    Serial  = 1 << 3,
    Input   = 1 << 4,
}
