# Semantic Infrastructure — LuaLaTeX Textbook Template

**Author:** Flyxion  
**Engine:** LuaLaTeX (requires TeX Live 2023+ or MiKTeX 23+)

---

## Directory Structure

```
textbook/
├── main.tex                  ← Master document
├── build.sh                  ← Build script (latexmk)
├── references.bib            ← BibLaTeX database
├── styles/
│   └── commands.tex          ← All custom commands & notation
├── chapters/
│   ├── preface.tex
│   ├── ch01-world-before-objects.tex   ← Full sample chapter
│   ├── ch02-regions-boundaries-collapse.tex
│   ├── appendix-a.tex
│   └── …
└── diagrams/
    ├── title-emblem.tex      ← RSVP field triple (3D TikZ)
    ├── constraint-nesting-3d.tex
    └── …
```

---

## Required Packages

All included in TeX Live full. Key packages:

| Package | Purpose |
|---------|---------|
| `fontspec` | LuaLaTeX font loading |
| `unicode-math` | Unicode math with TeX Gyre Pagella Math |
| `tcolorbox` | All aside/box environments |
| `tikz-3dplot` | 3D coordinate systems |
| `pgfplots` | Data plots and surfaces |
| `titlesec` | Chapter/section formatting |
| `biblatex + biber` | Bibliography |
| `imakeidx` | Index generation |
| `microtype` | Typographic refinement |

---

## Box Environments

| Environment | Use |
|-------------|-----|
| `\begin{definition}{Title}{label}` | Formal definitions (amber) |
| `\begin{theorem}{Title}{label}` | Theorems (deep teal) |
| `\begin{proposition}{Title}{label}` | Propositions (mid teal) |
| `\begin{example}` | Examples (gray) |
| `\begin{remark}` | Remarks/asides (purple left-bar) |
| `\begin{warning}` | Warnings (red left-bar) |
| `\begin{conceptaside}{Title}` | Major conceptual asides (purple) |
| `\begin{historynote}` | Historical notes (sepia) |
| `\begin{exercise}` | Chapter exercises (amber) |
| `\begin{namedthm}{Title}` | Named theorems (no counter) |
| `\begin{proof}` | Proofs with ■ QED |

---

## Custom Notation (styles/commands.tex)

| Command | Renders |
|---------|---------|
| `\rsvp` | $(\Phi, \mathbf{v}, S)$ |
| `\sphere{e}` | $\langle e \rangle$ |
| `\eval{e}` | $\llbracket e \rrbracket$ |
| `\collapse{e}` | $\downarrow\!e$ |
| `\pop` | $\bullet$ |
| `\Acc` | $\mathcal{A}$ |
| `\Adm` | $\mathbf{Adm}$ |
| `\admissible{C}` | $\mathrm{Adm}(C)$ |
| `\lamphron` | $\lambda_+$ |
| `\lamphrodyne` | $\lambda_-$ |
| `\sidenote{text}` | Italic margin note |
| `\sidedef{text}` | Bold amber margin note |
| `\chapterepigraph{quote}{attribution}` | Right-aligned epigraph |

---

## Building

```bash
chmod +x build.sh
./build.sh          # full build → main.pdf
./build.sh --clean  # clean artifacts
./build.sh --watch  # continuous compilation
```

Or manually:
```bash
lualatex -shell-escape main.tex
biber main
lualatex -shell-escape main.tex
lualatex -shell-escape main.tex
makeindex main
lualatex -shell-escape main.tex
```

---

## Adding New Chapters

1. Create `chapters/chNN-title.tex`
2. Add `\input{chapters/chNN-title}` in `main.tex` under the correct `\part{}`
3. Add new diagrams to `diagrams/` and `\input{}` them inside `\begin{figure}` environments

## Adding New 3D Diagrams

Each diagram is a standalone `.tex` file included via:
```latex
\begin{figure}[h]
  \centering
  \begin{tikzpicture}
    \input{diagrams/my-diagram}
  \end{tikzpicture}
  \caption{...}
  \label{fig:my-diagram}
\end{figure}
```

The diagram file should contain only TikZ/pgfplots code, no `\begin{tikzpicture}` wrapper.
