use rusteal_runtime::uenum;

#[uenum]
pub enum EGestureType {
    Translation,
    Rotation,
    Scale,
}

#[uenum]
pub enum EUIStyle {
    Light,
    Dark,
    Game,
}

#[uenum]
pub enum EFontTypeFace {
    Regular,
    Bold,
    Italic,
    Light,
}

#[uenum]
pub enum EIconType {
    Enable,
    Disable,
    Close,
    ArrowLeft,
    ArrowRight,
    CapsuleLeft,
    CapsuleCenter,
    CapsuleRight,
    Snapshot,
    Options,
    Reset,
    Info,
}

impl EUIStyle {
    pub const COUNT: i32 = 3;

    pub fn from_index(index: i32) -> Self {
        match index.rem_euclid(Self::COUNT) {
            0 => EUIStyle::Light,
            1 => EUIStyle::Dark,
            _ => EUIStyle::Game,
        }
    }

    pub fn index(self) -> i32 {
        self as i32
    }

    pub fn name(self) -> &'static str {
        match self {
            EUIStyle::Light => "Light",
            EUIStyle::Dark => "Dark",
            EUIStyle::Game => "Game",
        }
    }
}

impl EFontTypeFace {
    pub fn typeface(self) -> &'static str {
        match self {
            EFontTypeFace::Regular => "Regular",
            EFontTypeFace::Bold => "Bold",
            EFontTypeFace::Italic => "Italic",
            EFontTypeFace::Light => "Light",
        }
    }
}
