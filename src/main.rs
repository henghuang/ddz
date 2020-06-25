#![allow(warnings, unused)]

extern crate winapi;
use image::imageops::crop;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb, RgbImage, SubImage};
use std::ffi::OsStr;
use std::fs;
use std::iter::once;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use winapi::ctypes::{c_int, c_void};
use winapi::shared::minwindef::LPVOID;
use winapi::shared::ntdef::{LPCWSTR, PVOID};
use winapi::shared::windef::{HWND, LPRECT, RECT};
use winapi::um::wingdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
    SelectObject, BITMAPINFO, BITMAPINFOHEADER, LPBITMAPINFO, RGBQUAD,
};
use winapi::um::wingdi::{GetDeviceCaps, BI_RGB, DIB_RGB_COLORS, HORZRES, SRCCOPY, VERTRES};
use winapi::um::winuser::CF_BITMAP;
use winapi::um::winuser::{
    CloseClipboard, EmptyClipboard, FindWindowW, GetDC, GetWindowRect, OpenClipboard, ReleaseDC,
    SetClipboardData, SystemParametersInfoW, SPI_GETWORKAREA,
};

#[allow(unused)]
fn capture_screen(
    className: &str,
    winName: &str,
    screen_scale: i32,
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    unsafe {
        // copy screen to bitmap
        // Get the window handle of calculator application.
        let className: Vec<u16> = OsStr::new(className).encode_wide().chain(once(0)).collect();
        let winName: Vec<u16> = OsStr::new(winName).encode_wide().chain(once(0)).collect();
        let targetHWND = FindWindowW(0 as LPCWSTR, winName.as_ptr());
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        GetWindowRect(targetHWND, &mut rect);
        let scale = screen_scale; //screen scale
        let w = (rect.right - rect.left) * scale as c_int;
        let h = (rect.bottom - rect.top) * scale as c_int;
        let x1 = rect.left * scale as c_int;
        let y1 = rect.top * scale as c_int;

        let mut img = RgbImage::new(w as u32, h as u32);

        //get data
        let hScreen = GetDC(0 as HWND);
        let hDC = CreateCompatibleDC(hScreen);
        let hBitmap = CreateCompatibleBitmap(hScreen, w, h);
        let old_obj = SelectObject(hDC, hBitmap as *mut c_void);
        let bRet = BitBlt(hDC, 0, 0, w, h, hScreen, x1, y1, SRCCOPY);

        let bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 24,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };
        let bmiColors = [RGBQUAD {
            rgbBlue: 0,
            rgbGreen: 0,
            rgbRed: 0,
            rgbReserved: 0,
        }];
        let mut bitmapInform = BITMAPINFO {
            bmiHeader,
            bmiColors,
        };
        // the following calculations work for 16/24/32 bits bitmaps
        // but assume a byte pixel array
        let pixelSize = bitmapInform.bmiHeader.biBitCount / 8;
        let scanlineSize = ((pixelSize as u32) * (w as u32) + 3) & !3;
        let bitmapSize = (h as u32) * (scanlineSize as u32);

        let mut buffer = vec![0 as u8; bitmapSize as usize];
        GetDIBits(
            hDC,
            hBitmap,
            0,
            h as u32,
            buffer.as_mut_ptr() as LPVOID,
            &mut bitmapInform,
            DIB_RGB_COLORS,
        );

        for y in 0..h {
            for x in 0..w {
                let pixelOffset = (y * (scanlineSize as i32) + x * (pixelSize as i32)) as usize;
                img.put_pixel(
                    x as u32,
                    y as u32,
                    Rgb([
                        buffer[pixelOffset + 2],
                        buffer[pixelOffset + 1],
                        buffer[pixelOffset],
                    ]),
                );
            }
        }
        // save bitmap to clipboard
        // OpenClipboard(0 as HWND);
        // EmptyClipboard();
        // SetClipboardData(CF_BITMAP, hBitmap as *mut c_void);
        // CloseClipboard();

        // clean up
        SelectObject(hDC, old_obj);
        DeleteDC(hDC);
        ReleaseDC(0 as HWND, hScreen);
        DeleteObject(hBitmap as *mut c_void);
        return img;
    }
}

fn cropCardsViews(img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let mut leftBound = 0;
    let mut rightBound = 0;
    for i in 0..img.width() {
        let pixel = img.get_pixel(i, img.height() / 2);
        if pixel[0] > 240 && pixel[1] > 240 && pixel[2] > 240 {
            leftBound = i;
            break;
        }
    }
    for i in (0..img.width()).rev() {
        let pixel = img.get_pixel(i, img.height() / 2);
        if pixel[0] > 200 && pixel[1] > 200 && pixel[2] > 200 {
            rightBound = i;
            break;
        }
    }

    let x1 = leftBound;
    let y1 = 0;
    let width = rightBound - leftBound;
    let height = img.height();
    crop(img, x1 as u32, y1 as u32, width as u32, height as u32).to_image()
}
struct PlayerMe {
    viewAreaTopLeft: [f32; 2],
    viewAreaButtomRight: [f32; 2],
    viewOffset: f32,
}

impl PlayerMe {
    fn new() -> PlayerMe {
        PlayerMe {
            viewAreaTopLeft: [0.0141, 0.641],
            viewAreaButtomRight: [0.994, 0.78],
            viewOffset: 0.04487179487,
        }
    }
    fn getViewArea(
        &self,
        screen: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    ) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
        let x1 = screen.width() as f32 * self.viewAreaTopLeft[0];
        let y1 = screen.height() as f32 * self.viewAreaTopLeft[1];
        let x2 = screen.width() as f32 * self.viewAreaButtomRight[0];
        let y2 = screen.height() as f32 * self.viewAreaButtomRight[1];
        let width = x2 - x1;
        let height = y2 - y1;

        let mut viewImg =
            crop(screen, x1 as u32, y1 as u32, width as u32, height as u32).to_image();
        // viewImg.save("playerme_pre.jpg").unwrap();
        cropCardsViews(&mut viewImg)
    }
    fn getViewAreaEach(&self, screen: &mut ImageBuffer<Rgb<u8>, Vec<u8>>) {
        let mut cards = Vec::<ImageBuffer<Rgb<u8>, Vec<u8>>>::new();
        let cardOffset = screen.width() as f32 * self.viewOffset;
        let mut viewImage = self.getViewArea(screen);
        let viewImageHeight = viewImage.height();
        let cardNumber = viewImage.width() / cardOffset as u32;
        for cardIndex in 0..cardNumber {
            let xLocation = cardOffset as u32 * cardIndex;
            let eachCard = crop(
                &mut viewImage,
                xLocation,
                0,
                cardOffset as u32,
                viewImageHeight,
            );
            cards.push(eachCard.to_image());
        }
        // for (i, item) in cards.iter().enumerate() {
        //     let filename = format!("playerme{}.jpg", i);
        //     item.save(filename).unwrap();
        // }
    }
}

struct GroundTruth {
    groundTruthCards: Vec<ImageBuffer<Rgb<u8>, Vec<u8>>>,
}

impl GroundTruth {
    fn new() {
        let paths = fs::read_dir("./ground_truth").unwrap();
        for path in paths {
            println!("Name: {}", path.unwrap().path().display())
        }
    }
}
fn main() {
    let mut screenImg = capture_screen("", "雷电模拟器", 2);
    let playerme = PlayerMe::new();
    let img2 = playerme.getViewArea(&mut screenImg);
    // img2.save("playerme.jpg").unwrap();
    playerme.getViewAreaEach(&mut screenImg);
    let ground_truth = GroundTruth::new();
    // img.save("hello.jpg").unwrap();
}
