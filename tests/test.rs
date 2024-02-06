use piz;

#[test]
fn test() {
    let bytes = std::fs::read("/tmp/zip.zip").unwrap();
    eprintln!("{:?}", bytes.len());
    
    let iter = piz::Zip::new(&bytes).unwrap().central_dir_iter;
    for file in iter {
        let extra = file.extra_fields;
        let decomp_size = file.decompressed_size;
        let comp_method = file.compression_method;
        let filename = std::str::from_utf8(file.filename).unwrap();
        let comment = std::str::from_utf8(file.comment).unwrap();

        eprintln!("comp_method={:?} comment={:?} extra={:?} decomp_size={} filename={:?}", comp_method, comment, extra, decomp_size, filename);
    }

    panic!();
}
