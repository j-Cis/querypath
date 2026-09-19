/// Struktura konfigurująca opcje skanowania i dopasowywania w `querypath`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PathsOptions {
    /// Czy zachowywać katalogi nadrzędne dla dopasowanych plików/katalogów.
    pub keep_parent: bool,
    /// Czy ignorować wielkość liter we wzorcach.
    pub ignore_case: bool,
}

impl PathsOptions {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn keep_parent(mut self, keep: bool) -> Self {
        self.keep_parent = keep;
        self
    }

    #[must_use]
    pub fn ignore_case(mut self, ignore: bool) -> Self {
        self.ignore_case = ignore;
        self
    }
}