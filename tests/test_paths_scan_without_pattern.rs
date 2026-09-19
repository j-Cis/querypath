use querypath::{FsWalk, PathsEntry};

#[test]
fn test_scan_single_dir_without_patterns() {
    // Skanujemy folder ./tests/_/abc/ (zawiera tylko podfoldery mod/ i src/, brak plików)
    let entry = PathsEntry::build(["./tests/_/abc/"]).unwrap();
    let walk = FsWalk::scan(&entry).expect("Skanowanie powinno się powiąść.");

    assert_eq!(
        walk.files.len(),
        0,
        "Katalog ./tests/_/abc/ nie posiada żadnych plików."
    );
    assert_eq!(
        walk.dirs.len(),
        2,
        "Katalog ./tests/_/abc/ posiada dokładnie 2 podkatalogi."
    );

    let dir_paths: Vec<&str> = walk.dirs.iter().map(|d| d.str.as_str()).collect();
    assert_eq!(dir_paths, vec!["./tests/_/abc/mod/", "./tests/_/abc/src/"]);
}

#[test]
fn test_scan_hidden_structure_without_patterns() {
    // Skanujemy skomplikowany, ukryty folder ./tests/_/.gh/
    let entry = PathsEntry::build(["./tests/_/.gh/"]).unwrap();
    let walk = FsWalk::scan(&entry).expect("Skanowanie .gh powinno się powiąść.");

    // W Twoim drzewie ./tests/_/.gh/ znajduje się dokładnie 7 plików i 5 podkatalogów
    assert_eq!(
        walk.files.len(),
        7,
        "W .gh powinno znajdować się dokładnie 7 plików."
    );
    assert_eq!(
        walk.dirs.len(),
        5,
        "W .gh powinno znajdować się dokładnie 5 katalogów."
    );

    let file_paths: Vec<&str> = walk.files.iter().map(|f| f.str.as_str()).collect();

    // Sprawdzamy czy pliki zostały wykryte i znormalizowane
    assert!(file_paths.contains(&"./tests/_/.gh/.oiu"));
    assert!(file_paths.contains(&"./tests/_/.gh/aoe"));
    assert!(file_paths.contains(&"./tests/_/.gh/lib/dw/dwew/src/erw.wq"));
    assert!(file_paths.contains(&"./tests/_/.gh/lib/dw/wewe.rs"));
    assert!(file_paths.contains(&"./tests/_/.gh/lib/dw/wxw"));
    assert!(file_paths.contains(&"./tests/_/.gh/lib/jewe.rs"));
    assert!(file_paths.contains(&"./tests/_/.gh/wrt.rs"));
}

#[test]
fn test_scan_multiple_alternatives_without_patterns() {
    // Skanujemy alternatywę dwiema gałęziami: abc oraz def
    let entry = PathsEntry::build(["./tests/_/{abc|def}/"]).unwrap();
    let walk = FsWalk::scan(&entry).expect("Skanowanie alternatywy powinno się powiąść.");

    let dir_paths: Vec<&str> = walk.dirs.iter().map(|d| d.str.as_str()).collect();

    assert_eq!(
        dir_paths.len(),
        4,
        "W sumie powinny zostać odnalezione 4 katalogi z abc/ i def/."
    );
    assert_eq!(
        dir_paths,
        vec![
            "./tests/_/abc/mod/",
            "./tests/_/abc/src/",
            "./tests/_/def/app/",
            "./tests/_/def/src/",
        ]
    );
}

#[test]
fn podglad_skanu_w_konsoli() {
    let entry = PathsEntry::build(["{./tests/_/.gh/|./tests/_/abc}"]).unwrap();
    let walk = FsWalk::scan(&entry).expect("Skanowanie powinno się powiąść.");

    println!("\n=================================================");
    println!(" 📂 ZESKANOWANE KATALOGI ({})", walk.dirs.len());
    println!("-------------------------------------------------");
    for dir in &walk.dirs {
        println!("  {}", dir.str);
    }

    println!("\n 📄 ZESKANOWANE PLIKI ({})", walk.files.len());
    println!("-------------------------------------------------");
    for file in &walk.files {
        println!("  {}", file.str);
    }

    println!("\n 📊 STATYSTYKI SUROWEGO SKANOWANIA");
    println!("-------------------------------------------------");
    println!("  Katalogi ogółem: {}", walk.stat.count_dirs);
    println!("  Puste katalogi:  {}", walk.stat.count_empty_dirs);
    println!("  Pliki ogółem:    {}", walk.stat.count_files);
    println!("  Puste pliki:     {}", walk.stat.count_empty_files);
    println!("=================================================\n");
}
