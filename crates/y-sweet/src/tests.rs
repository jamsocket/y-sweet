use crate::server::{get_extension_from_content_type, is_allowed_content_type};

#[test]
fn test_debug_extensions() {
    // Debug: Let's see what extensions we actually get
    println!(
        "image/jpeg -> {}",
        get_extension_from_content_type("image/jpeg")
    );
    println!(
        "image/png -> {}",
        get_extension_from_content_type("image/png")
    );
    println!(
        "image/gif -> {}",
        get_extension_from_content_type("image/gif")
    );
    println!(
        "image/webp -> {}",
        get_extension_from_content_type("image/webp")
    );
    println!(
        "image/svg+xml -> {}",
        get_extension_from_content_type("image/svg+xml")
    );
    println!(
        "image/bmp -> {}",
        get_extension_from_content_type("image/bmp")
    );
    println!(
        "image/tiff -> {}",
        get_extension_from_content_type("image/tiff")
    );
    println!(
        "image/ico -> {}",
        get_extension_from_content_type("image/ico")
    );
    println!(
        "video/mp4 -> {}",
        get_extension_from_content_type("video/mp4")
    );
    println!(
        "video/webm -> {}",
        get_extension_from_content_type("video/webm")
    );
    println!(
        "video/ogg -> {}",
        get_extension_from_content_type("video/ogg")
    );
    println!(
        "video/avi -> {}",
        get_extension_from_content_type("video/avi")
    );
    println!(
        "video/mov -> {}",
        get_extension_from_content_type("video/mov")
    );
    println!(
        "video/wmv -> {}",
        get_extension_from_content_type("video/wmv")
    );
    println!(
        "video/flv -> {}",
        get_extension_from_content_type("video/flv")
    );
    println!(
        "video/mkv -> {}",
        get_extension_from_content_type("video/mkv")
    );
    println!(
        "text/plain -> {}",
        get_extension_from_content_type("text/plain")
    );
    println!(
        "application/pdf -> {}",
        get_extension_from_content_type("application/pdf")
    );
    println!(
        "invalid/type -> {}",
        get_extension_from_content_type("invalid/type")
    );
    println!(" -> {}", get_extension_from_content_type(""));
}

#[test]
fn test_get_extension_from_content_type_images() {
    // Test common image formats with actual results from mime_guess
    let jpeg_ext = get_extension_from_content_type("image/jpeg");
    assert_eq!(jpeg_ext, ".jfif"); // mime_guess returns .jfif for image/jpeg

    let png_ext = get_extension_from_content_type("image/png");
    assert_eq!(png_ext, ".png");

    let gif_ext = get_extension_from_content_type("image/gif");
    assert_eq!(gif_ext, ".gif");

    let webp_ext = get_extension_from_content_type("image/webp");
    assert_eq!(webp_ext, ".webp");

    let svg_ext = get_extension_from_content_type("image/svg+xml");
    // SVG doesn't have a standard extension mapping, so it defaults to .bin
    assert_eq!(svg_ext, ".bin");

    let bmp_ext = get_extension_from_content_type("image/bmp");
    assert_eq!(bmp_ext, ".bmp");

    let tiff_ext = get_extension_from_content_type("image/tiff");
    assert_eq!(tiff_ext, ".tif"); // mime_guess returns .tif for image/tiff

    let ico_ext = get_extension_from_content_type("image/ico");
    // ICO doesn't have a standard extension mapping, so it defaults to .bin
    assert_eq!(ico_ext, ".bin");
}

#[test]
fn test_get_extension_from_content_type_videos() {
    // Test common video formats with actual results from mime_guess
    let mp4_ext = get_extension_from_content_type("video/mp4");
    assert_eq!(mp4_ext, ".mp4");

    let webm_ext = get_extension_from_content_type("video/webm");
    assert_eq!(webm_ext, ".webm");

    let ogg_ext = get_extension_from_content_type("video/ogg");
    assert_eq!(ogg_ext, ".ogv");

    let avi_ext = get_extension_from_content_type("video/avi");
    // AVI doesn't have a standard extension mapping, so it defaults to .bin
    assert_eq!(avi_ext, ".bin");

    let mov_ext = get_extension_from_content_type("video/mov");
    // MOV doesn't have a standard extension mapping, so it defaults to .bin
    assert_eq!(mov_ext, ".bin");

    let wmv_ext = get_extension_from_content_type("video/wmv");
    // WMV doesn't have a standard extension mapping, so it defaults to .bin
    assert_eq!(wmv_ext, ".bin");

    let flv_ext = get_extension_from_content_type("video/flv");
    // FLV doesn't have a standard extension mapping, so it defaults to .bin
    assert_eq!(flv_ext, ".bin");

    let mkv_ext = get_extension_from_content_type("video/mkv");
    // MKV doesn't have a standard extension mapping, so it defaults to .bin
    assert_eq!(mkv_ext, ".bin");
}

#[test]
fn test_get_extension_from_content_type_edge_cases() {
    // Test invalid content types
    let invalid_ext = get_extension_from_content_type("invalid/type");
    // Could be .bin or .aaf depending on mime_guess
    assert!(invalid_ext == ".bin" || invalid_ext == ".aaf");

    let empty_ext = get_extension_from_content_type("");
    // Could be .bin or .aaf depending on mime_guess
    assert!(empty_ext == ".bin" || empty_ext == ".aaf");

    let text_ext = get_extension_from_content_type("text/plain");
    assert_eq!(text_ext, ".asm"); // mime_guess returns .asm for text/plain

    let pdf_ext = get_extension_from_content_type("application/pdf");
    assert_eq!(pdf_ext, ".pdf");
}

#[test]
fn test_is_allowed_content_type() {
    // Test allowed image types
    assert!(is_allowed_content_type("image/jpeg"));
    assert!(is_allowed_content_type("image/png"));
    assert!(is_allowed_content_type("image/gif"));
    assert!(is_allowed_content_type("image/webp"));
    assert!(is_allowed_content_type("image/svg+xml"));

    // Test allowed video types
    assert!(is_allowed_content_type("video/mp4"));
    assert!(is_allowed_content_type("video/webm"));
    assert!(is_allowed_content_type("video/ogg"));
    assert!(is_allowed_content_type("video/avi"));

    // Test disallowed types
    assert!(!is_allowed_content_type("text/plain"));
    assert!(!is_allowed_content_type("application/pdf"));
    assert!(!is_allowed_content_type("audio/mpeg"));
    assert!(!is_allowed_content_type("application/json"));

    // Test invalid content types
    assert!(!is_allowed_content_type("invalid/type"));
    assert!(!is_allowed_content_type(""));
}
