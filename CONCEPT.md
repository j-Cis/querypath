# koncepcja querypath

jest paczka którą możemy użyć w innych projektach jej celem jest zamknięcie pewnej kwesti w jednej paczce aby w róznych projektach nie pisać tego na nowo, za każdym razem.

## deklarowanie lokalizacji pracy `paths_entry.rs`

- jeśli nie jest podana jest brana ta w której się coś uruchamia 
- możemy podać pojedynczą np: `./`, `./abc/`, `../../abc/`, `A:/abc/`
- możemy użyć alternatyw w ścieżce np `./{abc|def}/` oznacza to `./abc/` oraz `./def/`
- alternatywne gałęzie mogą mieć różną głębokość np `./{abc/ao/|def/}` oznacza to `./abc/ao/` oraz `./def/`
- możemy podać też wiele ścieżek np `{./abc/|A:/ala/ma/kota/}

## deklarowanie wzorców scieżek `paths_patterns.rs`

## deklarowanie opcji `paths_options.rs`

- możemy zachować albo odrzucić puste katalogi
- możemy zachować albo odrzucić ukryte katalogi i lub te zaczynające się od kropek `.git` rzecz jasna z całą zawartością 
- możemy zachować albo odrzucić ukryte pliki i/lub te zaczynajace się od kropki np `.setting`
- możemy zachować albo odrzucić puste pliki
- możemy zachować albo odrzucić pliki binarne

## wykonywanie procesu skanowania i zwracanie rezultu `querypath.rs`