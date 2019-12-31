#[allow(dead_code)]
pub enum IRQ {
    VBlank  = (1 << 0),
    LCDC    = (1 << 1),
    Timer   = (1 << 2),
    Serial  = (1 << 3),
    Input   = (1 << 4),
}
