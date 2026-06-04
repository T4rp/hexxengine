use std::{ffi::CStr, fmt::Display, mem::MaybeUninit, sync::Arc};

use glam::I64Vec2;
use paidtype::freetype::{
    _bindgen_ty_2, FT_BBox, FT_Done_Face, FT_Done_FreeType, FT_Done_Glyph, FT_Err_Ok, FT_Error,
    FT_Error_String, FT_F26Dot6, FT_Face, FT_FaceRec, FT_Get_Char_Index, FT_Get_Glyph,
    FT_Get_Kerning, FT_Glyph_BBox_Mode__FT_GLYPH_BBOX_PIXELS, FT_Glyph_Get_CBox, FT_Init_FreeType,
    FT_Int32, FT_Kerning_Mode__FT_KERNING_DEFAULT, FT_Library, FT_Library_Version, FT_Load_Glyph,
    FT_Long, FT_New_Memory_Face, FT_Render_Glyph, FT_Render_Mode, FT_Set_Char_Size,
    FT_Set_Pixel_Sizes, FT_UInt, FT_ULong, FT_Vector,
};

use crate::shapes::Boundsi64;

#[derive(Debug)]
pub struct FreetypeError(pub u32);

macro_rules! ft_check {
    ($error_code:ident) => {
        if $error_code as _bindgen_ty_2 != FT_Err_Ok {
            return Err(FreetypeError($error_code as u32));
        }
    };
}

impl Display for FreetypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw_error_str = unsafe { FT_Error_String(self.0 as FT_Error) };
        if raw_error_str.is_null() {
            write!(f, "Unknown freetype error: FreetypeError({})", self.0)
        } else {
            let error_cst = unsafe { CStr::from_ptr(raw_error_str) };
            let error_str = error_cst.to_string_lossy();
            write!(f, "{}", error_str)
        }
    }
}

struct FreetypeLibraryInner {
    raw: FT_Library,
}

impl FreetypeLibraryInner {
    fn inner(&self) -> FT_Library {
        self.raw
    }
}

impl Drop for FreetypeLibraryInner {
    fn drop(&mut self) {
        unsafe { FT_Done_FreeType(self.raw.cast()) };
    }
}

pub struct GlyphBitmap<'a> {
    pub width: u32,
    pub rows: u32,
    pub buffer: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    pub width: i64,
    pub height: i64,
    pub hori_bearing_x: i64,
    pub hori_bearing_y: i64,
    pub hori_advance: i64,
    pub vert_bearing_x: i64,
    pub vert_bearing_y: i64,
    pub vert_advance: i64,
}

#[derive(Clone)]
pub struct FreetypeLibrary {
    // Arc :(
    library: Arc<FreetypeLibraryInner>,
}

impl FreetypeLibrary {
    pub fn new() -> Result<Self, FreetypeError> {
        let mut library = FreetypeLibraryInner {
            raw: std::ptr::null_mut(),
        };

        let error_code = unsafe { FT_Init_FreeType(&mut library.raw) };
        ft_check!(error_code);

        Ok(Self {
            library: Arc::new(library),
        })
    }

    pub fn raw(&self) -> FT_Library {
        self.library.raw
    }

    pub fn version(&self) -> (i32, i32, i32) {
        let (mut major, mut minor, mut patch) = (0, 0, 0);
        unsafe { FT_Library_Version(self.raw(), &mut major, &mut minor, &mut patch) };
        (major, minor, patch)
    }

    pub fn new_memory_face(
        &self,
        font_data: &[u8],
        face_index: usize,
    ) -> Result<Face, FreetypeError> {
        let font_data = font_data.to_vec();

        let mut face = std::ptr::null_mut();

        let error_code = unsafe {
            FT_New_Memory_Face(
                self.raw(),
                font_data.as_ptr(),
                font_data.len() as FT_Long,
                face_index as FT_Long,
                &mut face,
            )
        };
        ft_check!(error_code);

        let lib = self.clone();

        Ok(Face {
            face,
            _font_data: font_data,
            _lib: lib,
        })
    }
}

pub struct Face {
    face: FT_Face,
    _font_data: Vec<u8>,
    _lib: FreetypeLibrary,
}

impl Face {
    pub fn raw_rec(&self) -> &FT_FaceRec {
        unsafe { &*self.face }
    }

    pub fn family_name(&self) -> Option<&str> {
        unsafe {
            let ptr = self.raw_rec().family_name;
            if ptr.is_null() {
                None
            } else {
                CStr::from_ptr(ptr).to_str().ok()
            }
        }
    }

    pub fn style_name(&self) -> Option<&str> {
        unsafe {
            let ptr = self.raw_rec().style_name;
            if ptr.is_null() {
                None
            } else {
                CStr::from_ptr(ptr).to_str().ok()
            }
        }
    }

    pub fn set_char_size(
        &self,
        width: u64,
        height: u64,
        horizontal_res: u32,
        vertical_res: u32,
    ) -> Result<(), FreetypeError> {
        let error_code = unsafe {
            FT_Set_Char_Size(
                self.face,
                width as FT_F26Dot6,
                height as FT_F26Dot6,
                horizontal_res as FT_UInt,
                vertical_res as FT_UInt,
            )
        };
        ft_check!(error_code);

        Ok(())
    }

    pub fn set_pixel_sizes(&self, width: u32, height: u32) -> Result<(), FreetypeError> {
        let error_code =
            unsafe { FT_Set_Pixel_Sizes(self.face, width as FT_UInt, height as FT_UInt) };
        ft_check!(error_code);
        Ok(())
    }

    pub fn get_char_index(&self, char_code: u64) -> Option<u32> {
        let glyph_index = unsafe { FT_Get_Char_Index(self.face, char_code as FT_ULong) };
        if glyph_index == 0 {
            return None;
        }

        Some(glyph_index)
    }

    pub fn load_glyph(&self, glyph_index: u32, load_flags: u32) -> Result<(), FreetypeError> {
        let error_code =
            unsafe { FT_Load_Glyph(self.face, glyph_index as FT_UInt, load_flags as FT_Int32) };
        ft_check!(error_code);

        Ok(())
    }

    pub fn render_glyph(&self, render_mode: FT_Render_Mode) -> Result<(), FreetypeError> {
        let error_code = unsafe { FT_Render_Glyph(self.raw_rec().glyph, render_mode) };
        ft_check!(error_code);
        Ok(())
    }

    pub fn get_bitmap_data(&self) -> Option<GlyphBitmap<'_>> {
        let bitmap = unsafe { (*self.raw_rec().glyph).bitmap };
        let width = bitmap.width;
        let rows = bitmap.rows;

        let bitmap_size = (width * rows) as usize;

        if bitmap_size == 0 {
            return None;
        }

        let buffer = unsafe { std::slice::from_raw_parts(bitmap.buffer, bitmap_size) };

        Some(GlyphBitmap {
            width,
            rows,
            buffer,
        })
    }

    pub fn get_glyph_advance(&self) -> (i32, i32) {
        let glyph_slot = unsafe { *self.raw_rec().glyph };

        let advance_x = (glyph_slot.advance.x >> 6) as i32;
        let advance_y = (glyph_slot.advance.y >> 6) as i32;

        (advance_x, advance_y)
    }

    pub fn get_glyph_left_top(&self) -> (i32, i32) {
        let glyph_slot = unsafe { *self.raw_rec().glyph };

        let bitmap_left = glyph_slot.bitmap_left;
        let bitmap_top = glyph_slot.bitmap_top;

        (bitmap_left, bitmap_top)
    }

    pub fn get_glyph_cbox(&self) -> Result<Boundsi64, FreetypeError> {
        let glyph_slot = unsafe { self.raw_rec().glyph };

        let mut cbox = FT_BBox {
            xMin: 0,
            yMin: 0,
            xMax: 0,
            yMax: 0,
        };

        unsafe {
            let mut glyph = MaybeUninit::uninit();

            let err = FT_Get_Glyph(glyph_slot, glyph.as_mut_ptr());
            ft_check!(err);

            FT_Glyph_Get_CBox(
                *glyph.as_mut_ptr(),
                FT_Glyph_BBox_Mode__FT_GLYPH_BBOX_PIXELS,
                &mut cbox,
            );

            FT_Done_Glyph(*glyph.as_mut_ptr());
        };

        Ok(Boundsi64 {
            x_min: cbox.xMin,
            y_min: cbox.yMin,
            x_max: cbox.xMax,
            y_max: cbox.yMax,
        })
    }

    pub fn get_glyph_metrics(&self) -> GlyphMetrics {
        let glyph_slot = unsafe { self.raw_rec().glyph };
        let glyph_metrics = unsafe { (*glyph_slot).metrics };
        GlyphMetrics {
            width: glyph_metrics.width,
            height: glyph_metrics.height,
            hori_bearing_x: glyph_metrics.horiBearingX,
            hori_bearing_y: glyph_metrics.horiBearingY,
            hori_advance: glyph_metrics.horiAdvance,
            vert_bearing_x: glyph_metrics.vertBearingX,
            vert_bearing_y: glyph_metrics.vertBearingY,
            vert_advance: glyph_metrics.vertAdvance,
        }
    }

    pub fn get_glyph_kerning(
        &self,
        left_glyph: u32,
        right_glyph: u32,
    ) -> Result<I64Vec2, FreetypeError> {
        let mut kerning = FT_Vector { x: 0, y: 0 };

        unsafe {
            let err = FT_Get_Kerning(
                self.face,
                left_glyph,
                right_glyph,
                FT_Kerning_Mode__FT_KERNING_DEFAULT,
                &mut kerning,
            );
            ft_check!(err);
        };

        Ok(I64Vec2::new(kerning.x as i64, kerning.y as i64))
    }
}

impl Drop for Face {
    fn drop(&mut self) {
        unsafe { FT_Done_Face(self.face) };
    }
}

#[cfg(test)]
mod tests {
    use paidtype::freetype::{
        FT_LOAD_DEFAULT, FT_Render_Mode__FT_RENDER_MODE_MONO, FT_Render_Mode__FT_RENDER_MODE_NORMAL,
    };

    use super::FreetypeLibrary;

    const FONT_FILE: &[u8] = include_bytes!("../../../assets/unifont-17.0.03.otf");

    #[test]
    fn library_init() {
        FreetypeLibrary::new().unwrap();
    }

    #[test]
    fn library_version() {
        println!("loading library");
        let freetype = FreetypeLibrary::new().unwrap();

        let version = freetype.version();
        assert_eq!(version.0 != 0 || version.1 != 0 || version.2 != 0, true);
    }

    #[test]
    fn face_creation() {
        println!("loading library");
        let freetype = FreetypeLibrary::new().unwrap();

        println!("loading font");
        let face = freetype.new_memory_face(FONT_FILE, 0).unwrap();
        let family = face.family_name().unwrap();
        let style = face.style_name().unwrap();
        println!("{family} {style}");
    }

    #[test]
    fn render_glyph() {
        println!("loading library");
        let freetype = FreetypeLibrary::new().unwrap();

        println!("loading font");
        let face = freetype.new_memory_face(FONT_FILE, 0).unwrap();
        face.set_pixel_sizes(0, 16).unwrap();

        let glyph_index = face.get_char_index(0x40).unwrap();
        face.load_glyph(glyph_index, FT_LOAD_DEFAULT).unwrap();
        face.render_glyph(FT_Render_Mode__FT_RENDER_MODE_MONO)
            .unwrap();

        let loaded_index = unsafe { (*face.raw_rec().glyph).glyph_index };
        assert_eq!(loaded_index, glyph_index);

        let bitmap_data = face.get_bitmap_data().unwrap();
        assert_eq!(bitmap_data.width > 0, true);
        assert_eq!(bitmap_data.rows > 0, true);
    }

    #[test]
    fn library_dropping() {
        let ft1 = FreetypeLibrary::new().unwrap();
        let ft2 = ft1.clone();
        drop(ft2);
        drop(ft1);
    }

    #[test]
    fn face_dropping() {
        let freetype = FreetypeLibrary::new().unwrap();
        let face = freetype.new_memory_face(FONT_FILE, 0).unwrap();

        face.set_pixel_sizes(0, 16).unwrap();

        let glyph_index = face.get_char_index(0x40).unwrap();
        face.load_glyph(glyph_index, FT_LOAD_DEFAULT).unwrap();
        face.render_glyph(FT_Render_Mode__FT_RENDER_MODE_NORMAL)
            .unwrap();

        drop(freetype);
    }

    #[test]
    fn glyph_cbox() {
        let freetype = FreetypeLibrary::new().unwrap();
        let face = freetype.new_memory_face(FONT_FILE, 0).unwrap();
        face.set_pixel_sizes(0, 16).unwrap();
        let glyph_index = face.get_char_index(0x40).unwrap();
        face.load_glyph(glyph_index, FT_LOAD_DEFAULT).unwrap();

        let cbox = face.get_glyph_cbox().unwrap();
        println!("{:?}", cbox)
    }

    #[test]
    fn glyph_kerning() {
        let freetype = FreetypeLibrary::new().unwrap();

        let face = freetype.new_memory_face(FONT_FILE, 0).unwrap();
        face.set_pixel_sizes(0, 16).unwrap();

        let first_glyph = face.get_char_index('V' as u64).unwrap();
        let second_glyph = face.get_char_index('A' as u64).unwrap();

        let kerning = face.get_glyph_kerning(first_glyph, second_glyph).unwrap();
        println!("{:?}", kerning)
    }
}
