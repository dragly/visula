use crate::application::Application;
use crate::post_process::config::{
    BloomConfig, OutlineConfig, SkyConfig, SkyMode, SsaoConfig, Tonemapping,
};

pub struct RenderingControls {
    ssao_toggle_requested: Option<bool>,
    ssao_config: SsaoConfig,
    bloom_toggle_requested: Option<bool>,
    bloom_config: BloomConfig,
    sky_config_update: Option<SkyConfig>,
    tonemapping_update: Option<Tonemapping>,
    outline_config: OutlineConfig,
    initialized: bool,
}

impl RenderingControls {
    pub fn new() -> Self {
        Self {
            ssao_toggle_requested: None,
            ssao_config: SsaoConfig::default(),
            bloom_toggle_requested: None,
            bloom_config: BloomConfig::default(),
            sky_config_update: None,
            tonemapping_update: None,
            outline_config: OutlineConfig::default(),
            initialized: false,
        }
    }

    pub fn gui(&mut self, application: &Application, ui: &mut egui::Ui) {
        let config = &application.post_processor.config;

        ui.collapsing("Sky", |ui| {
            let current_mode = config.sky.mode;
            for mode in [SkyMode::Off, SkyMode::NormalMap, SkyMode::SkyGround] {
                if ui
                    .selectable_label(current_mode == mode, mode.to_string())
                    .clicked()
                {
                    self.sky_config_update = Some(SkyConfig { mode });
                }
            }
        });

        ui.collapsing("Tonemapping", |ui| {
            let current = config.tonemapping;
            for tm in [
                Tonemapping::None,
                Tonemapping::Reinhard,
                Tonemapping::AcesFilmic,
            ] {
                let label = match tm {
                    Tonemapping::None => "None",
                    Tonemapping::Reinhard => "Reinhard",
                    Tonemapping::AcesFilmic => "ACES Filmic",
                };
                if ui.selectable_label(current == tm, label).clicked() {
                    self.tonemapping_update = Some(tm);
                }
            }
        });

        ui.collapsing("SSAO", |ui| {
            let mut ssao_enabled = config.ssao.is_some();
            if ui.checkbox(&mut ssao_enabled, "Enabled").changed() {
                self.ssao_toggle_requested = Some(ssao_enabled);
            }

            ui.add(egui::Slider::new(&mut self.ssao_config.radius, 0.01..=5.0).text("Radius"));
            ui.add(egui::Slider::new(&mut self.ssao_config.bias, 0.001..=0.1).text("Bias"));
            ui.add(egui::Slider::new(&mut self.ssao_config.intensity, 0.0..=5.0).text("Intensity"));
        });

        ui.collapsing("Bloom", |ui| {
            let mut bloom_enabled = config.bloom.is_some();
            if ui.checkbox(&mut bloom_enabled, "Enabled").changed() {
                self.bloom_toggle_requested = Some(bloom_enabled);
            }

            ui.add(
                egui::Slider::new(&mut self.bloom_config.threshold, 0.0..=5.0).text("Threshold"),
            );
            ui.add(
                egui::Slider::new(&mut self.bloom_config.intensity, 0.0..=2.0).text("Intensity"),
            );
        });

        ui.collapsing("Outline", |ui| {
            ui.checkbox(&mut self.outline_config.enabled, "Enabled");
            ui.add(
                egui::Slider::new(&mut self.outline_config.thickness, 1.0..=10.0).text("Thickness"),
            );
            ui.add(
                egui::Slider::new(&mut self.outline_config.depth_threshold, 0.01..=2.0)
                    .text("Depth threshold"),
            );
            ui.horizontal(|ui| {
                ui.label("Color");
                ui.color_edit_button_rgb(&mut self.outline_config.color);
            });
        });
    }

    pub fn update(&mut self, application: &mut Application) {
        if !self.initialized {
            self.outline_config = application.post_processor.config.outline.clone();
            self.initialized = true;
        }
        if let Some(config) = self.sky_config_update.take() {
            application.post_processor.config.sky = config;
        }
        if let Some(tm) = self.tonemapping_update.take() {
            application.post_processor.config.tonemapping = tm;
        }
        if let Some(enable) = self.ssao_toggle_requested.take() {
            if enable {
                application.post_processor.config.ssao = Some(self.ssao_config.clone());
                application.post_processor.enable_ssao(
                    &application.device,
                    &application.queue,
                    application.config.width,
                    application.config.height,
                    &application.camera,
                    &application.depth_texture,
                );
            } else {
                application.post_processor.config.ssao = None;
                application.post_processor.disable_ssao(&application.device);
            }
        }
        if application.post_processor.config.ssao.is_some() {
            application.post_processor.config.ssao = Some(self.ssao_config.clone());
        }
        if let Some(enable) = self.bloom_toggle_requested.take() {
            if enable {
                application.post_processor.config.bloom = Some(self.bloom_config.clone());
                application.post_processor.enable_bloom(
                    &application.device,
                    application.config.width,
                    application.config.height,
                );
            } else {
                application.post_processor.config.bloom = None;
                application
                    .post_processor
                    .disable_bloom(&application.device);
            }
        }
        if application.post_processor.config.bloom.is_some() {
            application.post_processor.config.bloom = Some(self.bloom_config.clone());
        }
        application.post_processor.config.outline = self.outline_config.clone();
    }
}

impl Default for RenderingControls {
    fn default() -> Self {
        Self::new()
    }
}
