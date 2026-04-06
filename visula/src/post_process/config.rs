#[derive(Debug, Clone)]
pub struct PostProcessConfig {
    pub ssao: Option<SsaoConfig>,
    pub bloom: Option<BloomConfig>,
    pub tonemapping: Tonemapping,
    pub sky: SkyConfig,
    pub outline: OutlineConfig,
}

impl Default for PostProcessConfig {
    fn default() -> Self {
        Self {
            ssao: None,
            bloom: None,
            tonemapping: Tonemapping::AcesFilmic,
            sky: SkyConfig::default(),
            outline: OutlineConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OutlineConfig {
    pub enabled: bool,
    pub color: [f32; 3],
    pub thickness: f32,
    pub depth_threshold: f32,
}

impl Default for OutlineConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            color: [0.067, 0.102, 0.055],
            thickness: 3.0,
            depth_threshold: 0.1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SkyMode {
    Off,
    NormalMap,
    SkyGround,
}

impl std::fmt::Display for SkyMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SkyMode::Off => write!(f, "Off"),
            SkyMode::NormalMap => write!(f, "Normal Map"),
            SkyMode::SkyGround => write!(f, "Sky / Ground"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkyConfig {
    pub mode: SkyMode,
}

impl Default for SkyConfig {
    fn default() -> Self {
        Self {
            mode: SkyMode::NormalMap,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SsaoConfig {
    pub radius: f32,
    pub bias: f32,
    pub intensity: f32,
}

impl Default for SsaoConfig {
    fn default() -> Self {
        Self {
            radius: 1.5,
            bias: 0.025,
            intensity: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BloomConfig {
    pub threshold: f32,
    pub intensity: f32,
    pub mip_levels: u32,
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self {
            threshold: 0.8,
            intensity: 0.5,
            mip_levels: 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tonemapping {
    None,
    Reinhard,
    AcesFilmic,
}
