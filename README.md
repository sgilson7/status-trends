# Status Trends

Status Trends turns a CSV export from Jira, another tracker, or a gradebook into a dashboard: how many items came in each year, quarter or month, split by where they stand today (still requested, pending, completed), the percentage change on the period before and on the same period a year before, completion against a target, the open backlog, and a list of insights and risks worked out from the data.

**Use it:** https://sgilson7.github.io/status-trends/

![The title art: a street of buildings that rise left to right like a bar chart, a billboard with a stacked column chart, and a green robot cat looking at it](web/art/title.png)

## Where your data goes

The file you open is read with the browser's own file reader and analysed in the tab by a WebAssembly module. The site is a handful of static files on GitHub Pages; there is no server behind it that could receive data.

What is checked, in Chromium, Firefox and WebKit, on every run of `./scripts/check.sh` (`tests/browser/check.py` and `tests/browser/interactions.py`):

- After the page reports ready, it makes **no request of any kind**, to its own origin or any other, while a file is opened, the view is changed, statuses are moved, the table and a chart are saved, rows are pasted, both samples are opened and the data is cleared. The page fetches every file it needs before it reports ready, so a later request could only be the page sending something.
- Cookies, `localStorage`, `sessionStorage` and IndexedDB are empty at the end.
- No Content-Security-Policy violation, WebSocket, worker, service worker or popup.

What enforces it in the page itself: a Content-Security-Policy with `connect-src 'self'`, `form-action 'none'`, and no fonts, frames, workers or manifest; `referrer` set to `no-referrer`; no third-party script, font, analytics or account. The core boundary tests (`crates/core/tests/boundary.rs`, `no_storage.rs`) fail if the page's code names a network API other than `fetch`, a storage API, or another origin.

`./scripts/prove-privacy.sh` plants five leaks in a copy of the tool, one at a time (a same-origin request carrying data, `localStorage`, an image beacon, `sendBeacon`, a POST to another site), and requires the checks to catch each one. The last run caught all five.

What these checks do not cover: browser extensions, which can read any page you open; what you do with a file you save; and the request your browser makes to GitHub Pages for the page's own files, which carries your IP address the way any website visit does but none of your data. A pass shows what happened on the paths the checks drive; they do not drive drag-and-drop or Print, and a feature they never start is a feature they never saw. You can check the main claim yourself: open the page, turn off Wi-Fi, then open your file. The dashboard still works.

Whether your employer or school allows work data in a browser tool is a policy question this repository cannot answer.

## Getting your data out of Jira

Search for the issues you report on (Filters, then View all issues, or your project's issue search), then choose Export, then Export CSV. Status Trends needs a created date and a status for each row. A Resolved date turns on the completed and open-over-time charts and the time-to-complete figures; a Due date turns on the overdue check; a Team or Component column turns on the group table.

For a class or program, export one row per student per assignment, with the student, the date it was set, the due date, its status and the date it was turned in. The "Sample: a class" button shows the shape.

## What the dashboard computes

| Figure | Rule |
| --- | --- |
| Received | items created in the period, split by their status group today |
| Change | whole percent on the period before and the same period a year before, rounded half away from zero |
| Completion rate | completed ÷ received, leaving out excluded items (Won't Do, Duplicate, Excused) |
| Completed in period | items whose completion date falls in the period |
| Open at end | items created before the period's end and not completed by then |
| Median days to complete | for items completed in the period |
| In progress | the period holding the "as of" day (the latest date in the file unless you type one); comparisons use the last complete period |

The insights apply these rules with your thresholds (defaults: 80% completion target, 20% change alert, 90 days open): completion below target, completion falling three periods running, more received than completed three periods running, the open count jumping, items open too long, items past due, volume jumps, slower completion, and groups below target over the last year of complete periods.

Every column guess and status group can be changed under "Columns, statuses and thresholds". The year can start in any month; a fiscal year is named for the calendar year it ends in.

## How it is built

Status Trends is a [Builder](https://github.com/sgilson7/builder-setting) tool. Every rule above is in a Rust core (`crates/core`) with no input or output, tested with `cargo test`; a `wasm-bindgen` shim (`crates/shim`) forwards two calls and decides nothing; the page (`web/`) draws what the core returns, chart geometry included, and holds no number or arithmetic of its own. Page text is in `data/strings.json`.

```bash
./scripts/setup.sh          # once: Python venv with Playwright and three browsers
cargo test --workspace      # core, hand-computed and boundary tests, in seconds
./scripts/build.sh          # release WebAssembly build into dist/
python3 scripts/serve.py    # http://127.0.0.1:8000/
./scripts/check.sh          # everything, before a deploy
./scripts/prove-privacy.sh  # optional: plant five leaks, require each caught
./scripts/mutants.sh        # optional: mutation audit of the core
python3 tools/art.py        # rebuild the title art (pdflatex, pdftocairo)
python3 scripts/make_samples.py   # rebuild the invented sample data
```

Needs Rust with the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` 0.2.127, and Python 3.

The expected values in `crates/core/tests/hand_computed.rs` and `tests/browser/interactions.py` were worked out by hand from `tests/fixtures/small.csv`; the working is in the comments.

The sample data is invented by `scripts/make_samples.py` and describes no real team or class. The title art is drawn as TikZ by `tools/art.py`.

The build record is in `BRIEF.md`, `PLAN.md`, `SECOND-ORDER.md` and `HANDOFF.md`.

## License

MIT. See `LICENSE`.
