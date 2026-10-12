"""The project's own browser interactions, run by check.py in every engine.

check.py has already loaded the page, waited for it to report ready, and let
it idle. This drives the tool the way a person would: open a file, change
the view, move a status, save the table and a chart, paste rows, try both
samples, clear. check.py watches every request throughout, so these
interactions are also the path on which "no request after ready" is shown.

Expected values are worked out by hand from tests/fixtures/small.csv (C4);
the working is in crates/core/tests/hand_computed.rs. None is copied from
the tool's output.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
STRINGS = json.loads((ROOT / "data" / "strings.json").read_text())
UI = STRINGS["ui"]
SMALL = ROOT / "tests" / "fixtures" / "small.csv"
MINUS = "−"


def text(page, selector):
    return page.inner_text(selector).strip()


def row(page, table, label):
    """The cells of the row whose first cell is `label`."""
    cells = page.locator(f"#{table} tbody tr").filter(has=page.locator("th", has_text=label)).first.locator("th, td")
    return [c.strip() for c in cells.all_inner_texts()]


def choose(page, select_id, value):
    page.select_option(f"#{select_id}", value)


def run(page, check, engine):
    # The page's own text, from data/strings.json.
    check(text(page, "h1") == UI["title"], "the title from data/strings.json is drawn")
    check(UI["privacy_short"] in text(page, ".privacy-short"), "the privacy statement is drawn")
    check("undefined" not in page.inner_text("body"), "no label is missing from the strings file")
    check(page.is_visible("#empty") and page.is_hidden("#dashboard"), "before any data, only the empty state shows")

    # Open the small file through the file input.
    page.set_input_files("#file", str(SMALL))
    page.locator("#tiles .tile").first.wait_for()
    check(text(page, "#loaded-name") == "small.csv", "the opened file's name is shown")
    check(text(page, "#summary") == "12 requests from 3 Jan 2025 to 5 Apr 2026, read from 12 rows.",
          "the summary counts the twelve requests and the hand-worked date span")
    check(row(page, "period-table", "2026 Q1") == ["2026 Q1", "4", "1", "1", "2", "0", "—", "+33%", "50%", "2", "4", "15"],
          "2026 Q1 matches the hand-worked row")
    check(row(page, "period-table", "2026 Q2")[0] == "2026 Q2 (in progress)", "the quarter holding the as-of day is marked in progress")
    tile_values = page.locator(".tile-value").all_inner_texts()
    check(tile_values == ["4", "50%", "2", "5"], f"the four tiles show 4, 50%, 2 and 5 ({tile_values})")
    insights = page.locator(".insight-text").all_inner_texts()
    check("2 open requests are past the due date on 5 Apr 2026." in insights, "the overdue risk is listed")
    check(page.locator(".insight.risk").count() >= 1 and page.locator(".insight").first.get_attribute("class") == "insight risk",
          "risks are listed first")
    titles = page.locator(".chart h3").all_inner_texts()
    check(titles == [
        "Requests received per quarter, by status today",
        "Received and completed per quarter",
        "Completion rate per quarter",
        "Open at the end of each quarter",
        "Year over year: requests received per quarter",
    ], f"the five quarter charts are drawn ({titles})")
    labels = page.locator(".chart[data-key='received'] .bar-label").all_text_contents()
    check(labels == ["0%", MINUS + "100%", "so far"], f"the change over each column is on the chart ({labels})")
    group = row(page, "group-table", "Data")
    check(group == ["Data", "2", "—", "4", "2", "33%"], f"the Data team's row matches the hand-worked one ({group})")

    # A tooltip on hover.
    page.locator(".chart[data-key='received'] .hit-area").nth(4).hover()
    check(page.is_visible("#tip") and "2026 Q1" in text(page, "#tip"), "hovering a column shows its tooltip")
    page.mouse.move(0, 0)

    # Year view.
    choose(page, "granularity", "year")
    page.locator("#period-table tbody th", has_text="2026 (in progress)").wait_for()
    check(row(page, "period-table", "2025")[:2] == ["2025", "6"], "the year view counts six requests in 2025")
    check(row(page, "period-table", "2025")[7] == "60%", "2025's completion is 3 of 5 counted, 60%")
    check(page.locator(".chart[data-key='years']").count() == 0, "a year view has no year-over-year chart")
    choose(page, "granularity", "quarter")
    page.locator("#period-table tbody th", has_text="2026 Q1").wait_for()

    # Move a status to another group.
    page.click("#settings summary")
    page.locator("#statuses select[aria-label='In Review']").select_option("done")
    page.locator("#period-table tbody tr", has_text="75%").first.wait_for()
    check(row(page, "period-table", "2026 Q1")[8] == "75%", "moving In Review to completed makes 2026 Q1 3 of 4, 75%")
    page.locator("#statuses select[aria-label='In Review']").select_option("pending")
    page.locator("#period-table tbody tr", has_text="2026 Q1").filter(has_text="50%").wait_for()

    # A threshold.
    page.fill("#target", "50")
    page.press("#target", "Enter")
    page.locator("#target").blur()
    page.locator(".insight-text", has_text="meeting the 50% target").wait_for()
    check(True, "a 50% target turns the completion risk into good news")
    page.fill("#target", "")
    page.locator("#target").blur()
    page.locator(".insight-text", has_text="below the 80% target").wait_for()

    # Save the table, on this device.
    with page.expect_download() as d:
        page.click("#save-csv")
    lines = pathlib.Path(d.value.path()).read_bytes().decode("utf-8").split("\r\n")
    check(d.value.suggested_filename == "status-trends-by-quarter.csv", "the table is saved as status-trends-by-quarter.csv")
    check(lines[1] == "2025 Q1,3,0,1,2,0,,,67%,2,1,17", f"the saved table's first row matches the hand-worked one ({lines[1]})")

    # Save a chart.
    with page.expect_download() as d:
        page.locator(".chart[data-key='rate'] .chart-head button").click()
    svg = pathlib.Path(d.value.path()).read_bytes().decode("utf-8")
    check(svg.startswith("<svg") and "<style>" in svg and "Completion rate per quarter" in svg,
          "a chart is saved as an SVG that carries its own style and title")

    # Pasted rows.
    page.click(".paste summary")
    page.fill("#paste", "Created,Status\n2026-01-05,Done\n2026-02-05,Open\n")
    page.click("#use-paste")
    page.locator("#summary", has_text="2 requests").wait_for()
    check(text(page, "#summary") == "2 requests from 5 Jan 2026 to 5 Feb 2026, read from 2 rows.", "pasted rows are read")

    # A file with no dates: the column choices stay, with the problem named.
    page.fill("#paste", "Name,Status\nA,Open\n")
    page.click("#use-paste")
    page.locator("#problem:not([hidden])").wait_for()
    check(text(page, "#problem").startswith("No column looks like the date"), "a file with no date column says what to choose")
    check(page.is_visible("#settings select#created"), "the column choices stay open to fix it")

    # Text that is not CSV.
    page.fill("#paste", "a,\"b\n")
    page.click("#use-paste")
    check(text(page, "#message").startswith("A quoted field that starts on line 1"), "a broken file gets a message, not a crash")

    # Both samples.
    page.click("#sample-class")
    page.locator(".tile-label", has_text="Assignments set in").first.wait_for()
    check(page.input_value("#vocabulary") == "class" and page.input_value("#fiscal_start") == "8",
          "the class sample uses the class words and an August year")
    check(page.locator(".insight-text", has_text="Students below the 80% target").count() == 1, "the class sample names students below target")
    page.click("#sample-requests")
    page.locator(".tile-label", has_text="Requests received in").first.wait_for()
    check(page.locator(".chart").count() == 5, "the requests sample draws five charts")
    choose(page, "only_group", "Data")
    page.locator(".notes li", has_text="Showing only").wait_for()
    check(page.is_hidden("#group-section"), "showing one team hides the group table")
    choose(page, "granularity", "month")
    page.locator(".chart h3", has_text="per month").first.wait_for()
    check(page.locator(".chart[data-key='received'] .hit-area").count() > 1, "the month view draws a column per month")

    # Clear.
    page.click("#clear")
    check(page.is_hidden("#dashboard") and page.is_visible("#empty") and page.is_hidden("#loaded"), "Clear data removes the dashboard")
    check(page.input_value("#paste") == "", "Clear data empties the paste box")
