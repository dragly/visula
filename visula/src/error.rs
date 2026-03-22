use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Shader(#[from] visula_core::ShaderError),
    #[error("failed to create surface: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("no compatible GPU adapter found")]
    NoAdapter,
    #[error("failed to request GPU device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("surface not supported by adapter")]
    NoSurfaceConfig,
    #[error("failed to acquire surface texture: {0}")]
    SurfaceTexture(#[from] wgpu::SurfaceError),
    #[error("failed to parse font")]
    FontParse,
    #[error("tessellation failed: {0}")]
    Tessellation(#[from] lyon::tessellation::TessellationError),
    #[error("glTF error: {0}")]
    Gltf(#[from] gltf::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ZDF read error: {0}")]
    Zdf(String),
    #[error("window creation failed: {0}")]
    WindowCreation(#[from] winit::error::OsError),
    #[error("event loop error: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    #[error("missing binary data in glTF buffer")]
    GltfMissingBlobData,
}
