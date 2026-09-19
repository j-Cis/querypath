use querypath::PathsEntry;

#[test]
fn scenariusz_1_pojedyncza_lokalizacja() {
    let input = vec!["./tests/_/abc/"];
    let entry = PathsEntry::build(input)
        .expect("Scenariusz 1: Folder ./tests/_/abc/ istnieje i powinien zostać zmapowany.");

    assert_eq!(entry.targets.len(), 1);
    assert!(entry.targets[0].relative_path.ends_with("tests/_/abc/"));
    assert!(entry.targets[0].target_dir.str.ends_with("tests/_/abc"));
}

#[test]
fn scenariusz_2_wariant_ukryty() {
    let input = vec!["./tests/_/.gh/"];
    let entry = PathsEntry::build(input)
        .expect("Scenariusz 2: Ukryty folder ./tests/_/.gh/ istnieje i powinien zostać zmapowany.");

    assert_eq!(entry.targets.len(), 1);
    assert!(entry.targets[0].relative_path.ends_with("tests/_/.gh/"));
    assert!(entry.targets[0].target_dir.str.ends_with("tests/_/.gh"));
}

#[test]
fn scenariusz_3_alternatywa_zwykly_i_ukryty() {
    let input = vec!["./tests/_/{.gh|abc}/"];
    let entry = PathsEntry::build(input)
        .expect("Scenariusz 3: Obie gałęzie (.gh oraz abc) istnieją na dysku.");

    assert_eq!(
        entry.targets.len(),
        2,
        "Powinny zostać zmapowane dokładnie 2 lokalizacje."
    );

    let paths: Vec<&str> = entry
        .targets
        .iter()
        .map(|t| t.relative_path.as_str())
        .collect();
    assert!(paths.iter().any(|p| p.ends_with("tests/_/.gh/")));
    assert!(paths.iter().any(|p| p.ends_with("tests/_/abc/")));
}

#[test]
fn scenariusz_4_alternatywa_rozne_glebokosci() {
    let input = vec!["./tests/_/{.gh/lib/dw|abc}/"];
    let entry = PathsEntry::build(input)
        .expect("Scenariusz 4: Obie gałęzie o różnych głębokościach istnieją.");

    assert_eq!(entry.targets.len(), 2);

    let paths: Vec<&str> = entry
        .targets
        .iter()
        .map(|t| t.relative_path.as_str())
        .collect();
    assert!(paths.iter().any(|p| p.ends_with("tests/_/.gh/lib/dw/")));
    assert!(paths.iter().any(|p| p.ends_with("tests/_/abc/")));
}

#[test]
fn scenariusz_5_alternatywa_z_niewlasciwa_sciezka() {
    // Pierwsza ścieżka istnieje  (./tests/_/.gh/lib/dw/),
    // druga ścieżka NIE istnieje (./tests/_/.gh/lib/dw/abc/).
    let input = vec!["{./tests/_/.gh/lib/dw/|./tests/_/.gh/lib/dw/abc/}"];
    let entry = PathsEntry::build(input)
        .expect("Scenariusz 5: Powinien przetrwać istniejący cel, a błędny zostać odrzucony.");

    assert_eq!(
        entry.targets.len(),
        1,
        "Nieistniejący podfolder dw/abc/ musi zostać cicho odrzucony."
    );
    assert!(
        entry.targets[0]
            .relative_path
            .ends_with("tests/_/.gh/lib/dw/")
    );
}
