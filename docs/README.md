# ev documentation

This directory holds private development notes. The public documentation of
record for ev lives in the ssccs corpus, at `ssccs/docs/projects/ev/`
(`index.qmd`, `cva6.qmd`, `ibex.qmd`, `partnerships/exa-aws.qmd`), published
as https://docs.ssccs.org/projects/ev/. The Claude-readable artifact
published to `s3://ssccs-nexus-af/ev/docs` comes from that corpus as well.

The page sources and the publishing pipeline that used to live here were
removed in issue #51, because they duplicated the corpus and pointed at
pages that had already moved: the Quarto website configuration with its
`ev.ssccs.org` site URL and sidebar, the `sdb build` scripts, the
`Build Docs` workflow with its Cloudflare Pages deployment and artifact
sync, the shared `_include/` partials, and `arch/generate.py`, which
generated a page that consumed those partials.

What remains is `devlog/`: dated records of engineering decisions and their
measurements, one per task subject. Nothing in this directory is published,
so add public documentation to the ssccs corpus instead.
