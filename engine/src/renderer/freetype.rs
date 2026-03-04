use std::{error::Error, ffi::CStr, fmt::Display, sync::Arc};

use paidtype::freetype::{
    FT_Bitmap_Size, FT_Done_Face, FT_Done_FreeType, FT_Err_Ok, FT_Error_String, FT_Face,
    FT_FaceRec, FT_Get_Char_Index, FT_Init_FreeType, FT_Library, FT_Library_Version, FT_Load_Glyph,
    FT_New_Memory_Face, FT_Render_Glyph, FT_Render_Mode, FT_Set_Char_Size, FT_Set_Pixel_Sizes,
};

#[derive(Debug)]
pub struct FreetypeError(pub u32);

macro_rules! ft_check {
    ($error_code:ident) => {
        if $error_code as u32 != FT_Err_Ok {
            return Err(FreetypeError($error_code as u32));
        }
    };
}

impl Display for FreetypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw_error_str = unsafe { FT_Error_String(self.0 as i32) };
        if raw_error_str.is_null() {
            write!(f, "Unknown freetype error: FreetypeError({})", self.0)
        } else {
            let error_cst = unsafe { CStr::from_ptr(raw_error_str) };
            let error_str = error_cst.to_string_lossy();
            write!(f, "{}", error_str)
        }
    }
}

impl Error for FreetypeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
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
        let mut face = std::ptr::null_mut();
        let error_code = unsafe {
            FT_New_Memory_Face(
                self.raw(),
                font_data.as_ptr(),
                font_data.len() as i64,
                face_index as i64,
                &mut face,
            )
        };
        ft_check!(error_code);

        Ok(Face {
            face,
            _lib: self.clone(),
        })
    }
}

pub struct Face {
    face: FT_Face,
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
                width as i64,
                height as i64,
                horizontal_res,
                vertical_res,
            )
        };
        ft_check!(error_code);

        Ok(())
    }

    pub fn set_pixel_sizes(&self, width: u32, height: u32) -> Result<(), FreetypeError> {
        let error_code = unsafe { FT_Set_Pixel_Sizes(self.face, width, height) };
        ft_check!(error_code);
        Ok(())
    }

    pub fn get_char_index(&self, char_code: u64) -> Option<u32> {
        let glyph_index = unsafe { FT_Get_Char_Index(self.face, char_code) };
        if glyph_index == 0 {
            return None;
        }

        Some(glyph_index)
    }

    pub fn load_glyph(&self, glyph_index: u32, load_flags: u32) -> Result<(), FreetypeError> {
        let error_code = unsafe { FT_Load_Glyph(self.face, glyph_index, load_flags as i32) };
        ft_check!(error_code);

        Ok(())
    }

    pub fn render_glyph(&self, render_mode: FT_Render_Mode) -> Result<(), FreetypeError> {
        let error_code = unsafe { FT_Render_Glyph(self.raw_rec().glyph, render_mode) };
        ft_check!(error_code);
        Ok(())
    }
}

impl Drop for Face {
    fn drop(&mut self) {
        unsafe { FT_Done_Face(self.face) };
    }
}

#[cfg(test)]
mod tests {
    use paidtype::freetype::{FT_LOAD_DEFAULT, FT_Render_Mode__FT_RENDER_MODE_MONO};

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
    }
}
