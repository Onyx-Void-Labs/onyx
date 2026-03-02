use {
    super::{
        font::FontId,
        font_family::{FontFamily, FontFamilyId},
        image::{Bgra, Image},
        layouter::{self, LaidoutText, LayoutParams, Layouter},
        loader::{FontDefinition, FontFamilyDefinition},
        msdfer::Msdfer,
        rasterizer::{CompletedMsdfJob, OutlineRasterizationMode, QueuedMsdfJob, Rasterizer},
    },
    crate::makepad_platform::*,
    std::{cell::RefCell, mem, rc::Rc},
};

pub struct Fonts {
    layouter: Layouter,
    needs_prepare_atlases: bool,
    atlas_texture: Texture,
    msdf_job_sender: FromUISender<QueuedMsdfJob>,
    msdf_result_receiver: ToUIReceiver<CompletedMsdfJob>,
}

impl Fonts {
    pub fn new(cx: &mut Cx, settings: layouter::Settings) -> Self {
        let layouter = Layouter::new(settings);
        let (atlas_size, msdfer_settings) = {
            let rasterizer = layouter.rasterizer().borrow();
            (
                rasterizer.color_atlas().size(),
                rasterizer.msdfer().settings(),
            )
        };

        let mut msdf_job_sender: FromUISender<QueuedMsdfJob> = Default::default();
        let msdf_result_receiver: ToUIReceiver<CompletedMsdfJob> = Default::default();
        let worker_rx = msdf_job_sender.receiver();
        let worker_tx = msdf_result_receiver.sender();
        cx.spawn_thread(move || {
            let mut msdfer = Msdfer::new(msdfer_settings);
            while let Ok(job) = worker_rx.recv() {
                let mut msdf = Image::<Bgra>::new(job.key.size);
                msdfer.outline_to_msdf(
                    &job.outline,
                    job.dpxs_per_em,
                    &mut msdf.subimage_mut(super::geom::Rect::from(job.key.size)),
                );
                if worker_tx
                    .send(CompletedMsdfJob {
                        key: job.key,
                        pixels: msdf.into_pixels(),
                        epoch: job.epoch,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });

        Self {
            layouter,
            needs_prepare_atlases: false,
            // use single-channel texture for font atlas (R8).
            // This cuts memory by ~75% relative to VecBGRAu8_32.
            atlas_texture: Texture::new_with_format(
                cx,
                TextureFormat::VecRu8 {
                    width: atlas_size.width,
                    height: atlas_size.height,
                    data: None,
                    unpack_row_length: None,
                    updated: TextureUpdated::Empty,
                },
            ),
            msdf_job_sender,
            msdf_result_receiver,
        }
    }

    pub fn rasterizer(&self) -> &Rc<RefCell<Rasterizer>> {
        self.layouter.rasterizer()
    }

    pub fn outline_rasterization_mode(&self) -> OutlineRasterizationMode {
        self.layouter
            .rasterizer()
            .borrow()
            .outline_rasterization_mode()
    }

    pub fn set_outline_rasterization_mode(&mut self, mode: OutlineRasterizationMode) {
        self.layouter
            .rasterizer()
            .borrow_mut()
            .set_outline_rasterization_mode(mode);
    }

    pub fn grayscale_texture(&self) -> &Texture {
        &self.atlas_texture
    }

    pub fn color_texture(&self) -> &Texture {
        &self.atlas_texture
    }

    pub fn msdf_texture(&self) -> &Texture {
        &self.atlas_texture
    }

    pub fn is_font_family_known(&self, id: FontFamilyId) -> bool {
        self.layouter.is_font_family_known(id)
    }

    pub fn is_font_family_complete(&self, id: FontFamilyId) -> bool {
        self.layouter
            .loader
            .font_family_definitions
            .get(&id)
            .map(|def| def.font_ids.len() == def.expected_member_count)
            .unwrap_or(false)
    }

    pub fn is_font_known(&self, id: FontId) -> bool {
        self.layouter.is_font_known(id)
    }

    pub fn define_font_family(&mut self, id: FontFamilyId, definition: FontFamilyDefinition) {
        self.layouter.define_font_family(id, definition);
    }

    pub fn set_font_family_definition(
        &mut self,
        id: FontFamilyId,
        definition: FontFamilyDefinition,
    ) {
        self.layouter.set_font_family_definition(id, definition);
    }

    pub fn define_font(&mut self, id: FontId, definition: FontDefinition) {
        self.layouter.define_font(id, definition);
    }

    pub fn get_or_load_font_family(&mut self, id: FontFamilyId) -> Rc<FontFamily> {
        self.layouter.get_or_load_font_family(id)
    }

    pub fn get_or_layout(&mut self, params: impl LayoutParams) -> Rc<LaidoutText> {
        self.layouter.get_or_layout(params)
    }

    pub fn prepare_textures(&mut self, cx: &mut Cx) -> bool {
        assert!(!self.needs_prepare_atlases);
        let mut rasterizer = self.layouter.rasterizer().borrow_mut();
        if rasterizer.color_atlas_mut().reset_if_needed() {
            rasterizer.on_atlas_reset();
            return false;
        }
        drop(rasterizer);
        let completed = self.apply_completed_msdf_jobs();
        if completed > 0 {
            cx.redraw_all();
        }
        self.dispatch_msdf_jobs();
        self.prepare_atlas_texture(cx);
        self.needs_prepare_atlases = true;
        true
    }

    fn prepare_atlas_texture(&mut self, cx: &mut Cx) {
        let mut rasterizer = self.layouter.rasterizer().borrow_mut();
        let dirty_rect = rasterizer.color_atlas_mut().take_dirty_image().bounds();
        // original rasterizer output is Vec<u32> BGRA; convert to u8 grayscale
        let pixels_u32: Vec<u32> = unsafe {
            mem::transmute(rasterizer.color_atlas_mut().replace_pixels(Vec::new()))
        };
        let pixels_u8: Vec<u8> = pixels_u32
            .into_iter()
            .map(|px| (px & 0xff) as u8) // take blue channel (or replace with any)
            .collect();
        self.atlas_texture.put_back_vec_u8(
            cx,
            pixels_u8,
            Some(RectUsize::new(
                PointUsize::new(dirty_rect.origin.x, dirty_rect.origin.y),
                SizeUsize::new(dirty_rect.size.width, dirty_rect.size.height),
            )),
        )
    }

    pub fn prepare_atlases_if_needed(&mut self, cx: &mut Cx) {
        if !self.needs_prepare_atlases {
            return;
        }
        self.prepare_atlas(cx);
        self.needs_prepare_atlases = false;
    }

    fn prepare_atlas(&mut self, cx: &mut Cx) {
        let mut rasterizer = self.layouter.rasterizer().borrow_mut();
        let pixels_u8 = self.atlas_texture.take_vec_u8(cx);
        // expand back to u32 BGRA so rasterizer can consume it if needed
        let pixels_u32: Vec<u32> = pixels_u8
            .into_iter()
            .map(|b| ((0xff as u32) << 24) | ((b as u32) << 16) | ((b as u32) << 8) | (b as u32))
            .collect();
        unsafe {
            rasterizer
                .color_atlas_mut()
                .replace_pixels(mem::transmute(pixels_u32))
        };
    }

    fn dispatch_msdf_jobs(&mut self) {
        let jobs = self
            .layouter
            .rasterizer()
            .borrow_mut()
            .take_queued_msdf_jobs();
        for job in jobs {
            let _ = self.msdf_job_sender.send(job);
        }
    }

    fn apply_completed_msdf_jobs(&mut self) -> usize {
        let mut completed = 0usize;
        while let Ok(job) = self.msdf_result_receiver.try_recv() {
            self.layouter
                .rasterizer()
                .borrow_mut()
                .apply_completed_msdf_job(job);
            completed += 1;
        }
        completed
    }
}
