#set page(paper: "us-letter", margin: (x: 1.5cm, top: 1.5cm, bottom: 1.7cm),
  footer: context [
    #set text(size: 7.5pt, fill: luma(55%))
    #line(length: 100%, stroke: 0.5pt + luma(78%))
    #v(2pt)
    #align(right)[#counter(page).display() / #counter(page).final().first()]
  ])
#set text(font: ("Helvetica Neue", "Arial"), size: 10pt, number-width: "tabular")
#show raw: set text(size: 7.5pt, fill: luma(50%))
#let sparkbar(p) = box(width: 80%, height: 0.32em, fill: luma(90%))[#box(width: p * 1%, height: 100%, fill: rgb("#235a68"))]

#grid(columns: (1fr, auto), align: (left + bottom, right + bottom), column-gutter: 12pt,
  text(size: 18pt, weight: "bold")[Friday Night Program],
  text(size: 9.5pt, fill: luma(40%))[MEPS (0) · original · 3 cues · 5:08.2],
)
#v(5pt)
#line(length: 100%, stroke: 1pt)
#v(6pt)

#table(
  columns: (auto, auto, 1fr, 3cm, auto),
  stroke: none,
  inset: (x: 8pt, y: 9pt),
  align: (left + horizon, center + horizon, left + horizon, left + horizon, center + horizon),
  table.header(
    [], [],
    text(size: 7.5pt, fill: luma(45%), tracking: 0.5pt)[CUE], text(size: 7.5pt, fill: luma(45%), tracking: 0.5pt)[DURATION], text(size: 7.5pt, fill: luma(45%), tracking: 0.5pt)[AFTER],
  ),
  table.hline(stroke: 0.6pt + luma(55%)),
  [#text(fill: rgb("#235a68"), weight: "bold", size: 12pt)[1]], [], [#text(weight: 500)[Opening video] \ #raw("pub-mwbv track 5")], [#stack(spacing: 3.5pt, [2:00.2], sparkbar(81.7), text(size: 7pt, fill: luma(62%))[elapsed 2:00.2])], [#text(fill: luma(50%))[continue]],
  table.hline(stroke: 0.3pt + luma(88%)),
  [#text(fill: rgb("#235a68"), weight: "bold", size: 12pt)[2]], [#image("thumbs/02.png", width: 2cm)], [#text(weight: 500)[Chart: \[Section 2\] \*important\*] \ #raw("chart.png")], [#stack(spacing: 3.5pt, [0:08.0], sparkbar(21.1), text(size: 7pt, fill: luma(62%))[elapsed 2:08.2])], [#text(fill: luma(50%))[freeze]],
  table.hline(stroke: 0.3pt + luma(88%)),
  [#text(fill: rgb("#235a68"), weight: "bold", size: 12pt)[3]], [], [#text(weight: 500)[Closing song] \ #raw("sjjm track 151")], [#stack(spacing: 3.5pt, [3:00.0], sparkbar(100.0), text(size: 7pt, fill: luma(62%))[elapsed 5:08.2])], [#text(fill: luma(50%))[stop]],
  table.hline(stroke: 0.3pt + luma(88%)),
)
