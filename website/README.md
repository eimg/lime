# Lime guide website

Bilingual user documentation built with Astro and Starlight. English lives under
`/lime/`; Myanmar under `/lime/mm/` with HTML language `my`. The website has its
own package files and does not participate in Rust builds or app packaging.

## Local review

Use Node.js 24.14 or newer. From `website/`:

```sh
npm ci
npm run check
npm run build
npm run preview
```

Open <http://127.0.0.1:4173/lime/> or
<http://127.0.0.1:4173/lime/mm/>. Stop a foreground server with Ctrl+C.
If Astro starts it in the background, use `npm run preview:stop` (or
`npm run dev:stop` for the development server).
`npm run dev` provides live content reloads, but search requires the built preview.
Local preview keeps the `/lime/` prefix to catch GitHub Pages path errors.

## Content and translation

- `src/content/docs/`: English Markdown and MDX articles.
- `src/content/docs/mm/`: matching Myanmar articles with identical paths.
- `src/navigation.mjs`: shared page inventory and translated navigation groups.
- `src/content/i18n/my.json`: Myanmar interface and search labels.
- `src/components/GuideHome.astro`: paired homepage labels and links.
- `src/styles/custom.css`: charcoal/navy and pale blue from Lime's app/icon,
  warm gold focus details, and locally bundled Myanmar font.
- `src/assets/`: shared screenshots and explanatory illustrations.

Write about the published Lime release, using actual menu/control labels. The
current guide covers 1.3.0. Use plain, concise Burmese for instructions and retain
familiar software/editing terms in English. This applies to navigation, headings,
search labels, captions, and prose, not only literal menu paths. Keep ordinary
sentences natural instead of mechanically replacing every Burmese word.

Keep the published guide focused on using Lime: tasks, controls, outcomes, and
troubleshooting. Describe current behavior directly, without version labels,
release change summaries, developer notes, or work logs. Release history belongs
in GitHub release notes; build and publishing instructions belong in this README.

| Term or context | Myanmar editorial convention |
| --- | --- |
| Tools, Tool Options, Toolbox, Layers, Paths, Workspace, Canvas, Menu, Tab | Keep these familiar names in English; preserve exact UI labels |
| Selection, Mask, Brush, Font, Text, Opacity, Transparency, Transparent | Keep the technical term in English; explain its behavior in plain Burmese |
| Antialias / Antialias edges | Keep in English; do not substitute a Burmese phrase for softness |
| Feather / Soft edges | Keep these terms distinct from Antialias; Feather is a separate operation |
| Transform, Crop, Scale, Resize, Preview, Apply, Cancel, Undo, Redo, Save, Export | Keep in English when referring to editing operations or controls |
| Drop Shadow, Stroke, Outline, Effects, Filters | Keep the familiar effect names in English |
| Painting/drawing as an activity | Use **ပုံဆွဲခြင်း** |
| Official | Omit the qualifier from Myanmar prose; for example, link to **Releases** |
| Digital signing / notarization | Use **Signed**, **Unsigned**, and **Notarized** |
| Theme selector | Always use **Light**, **Dark**, **Auto**, and **Select theme** in English, in both language versions |

Explain editing concepts with concrete outcomes. Separate complete workflows from control reference details.
Keep consequences beside their actions and link to prerequisite concepts.

Every article needs `title`, `description`, `translationKey`, `sourceRevision`,
and `translatedRevision` frontmatter. Use matching permanent H2 IDs:

```md
## Choose a layer {#steps}
```

Translate the heading but preserve `{#steps}`. Use a different ID per section.
Keep matching section IDs in the same order in both languages. A custom remark
plugin removes the ID notation and emits normal HTML anchors. Language links
retain the article and, with JavaScript, the current section. Basic language
navigation works without JavaScript.

For links between articles at the two-level paths used here, write
`../../basics/layers/` rather than `/basics/layers/`; this retains both the
project prefix and current language. Shared image imports are relative to the
source file, so Myanmar files need one additional `../`.

When changing behavior, update both articles and advance the revision fields
together. Review both versions, then run:

```sh
npm run translations:record
npm run check
npm run build
```

`translations.json` records both files' SHA-256 hashes. Any later edit requires
recording again, making stale translation state visible. Recording acknowledges
the current files; it cannot assess translation quality. Content checks reject
missing counterparts, untranslated bodies, unlisted pages, mismatched sections,
and stale revision metadata. Build checks verify every article's links, section
targets, assets, language metadata, and both search indexes.

## Visuals and search

`workspace.jpg` is the repository's actual Lime screenshot, shared between
languages. `mask-example.svg` is a labeled-in-prose schematic, not an app capture.
Keep instructional images visible beside their procedure. Use descriptive alt
text and captions in both languages. Astro creates optimized local images.
Noto Sans Myanmar is bundled through Fontsource under the SIL Open Font License;
the distributed license is in `public/fonts/OFL.txt`. No external font service is needed.

Pagefind runs in the browser against static per-language indexes; it requires no
search server or account. Test both English tool labels and natural Myanmar
queries when adding content. Myanmar stemming is not supplied by Pagefind, so
do not assume English-style inflection matching. Navigation/content remain
available without JavaScript; search requires it.

Before publication, review the home, a long article, search, language switching,
light/dark themes, and mobile navigation in both languages. Check Myanmar line
height and word wrapping, keyboard focus, image captions, and active page links.

## GitHub Pages

The public addresses are:

- <https://eimg.github.io/lime/>
- <https://eimg.github.io/lime/mm/>

GitHub Pages serves the built static files from the root of the `gh-pages`
branch. There is no custom publishing workflow. The Markdown and site source
stay on `main`; `gh-pages` contains generated output only.

After reviewing, committing, and pushing source changes to `main`, run from
`website/`:

```sh
npm run publish:pages
```

This builds and checks the guide, then pushes `website/dist` to `gh-pages` with
a `.nojekyll` file so GitHub serves assets such as `_astro` directly. It uses your
normal Git credentials and leaves the current source branch in place. The
desktop app's `dist/` is separate. GitHub Pages picks up the branch update.

The repository's **Settings → Pages** source is **Deploy from a branch**, with
**gh-pages** and **/ (root)** selected. This setup is needed only once.

Update `site` and `base` in `astro.config.mjs` and the prefix checks if moving to
a different hosting path. No Sites registration or hosted application service
is required for this static GitHub Pages site.
