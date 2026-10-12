# Brief

Stage 1, written by the person. `C1 Frame the problem and set the rules`, `B1 Establish context`.

## The request, word for word

The tool request, as the person who will use it described it. The message that carried it also named the person and their employer; that framing is kept off this public repository on purpose (`SECOND-ORDER.md` row 1).

> We use a ticketing system to track our work called Jira. part of what I wanna see is number of requests per year based on different statuses like still requested still pending versus completed. I want to be able to create a dashboard in real time to see how growth compares across years or quarters or perhaps even months, depending on what I'm reporting I also want that it to provide insights and identify risks, so to show percentage increased decrease on the graphs, or trends that could show lack of KPI completion.
>
> Could be useful or transferable to teachers too, since they could track how students are doing within a certain program, for example, and be able to create dashboards based on how our students doing where the risks are things progressing in the right way etc.

The instructions that came with it, in substance: build on the Builder template; build the MVP and deploy it as a public repository on GitHub Pages and on sgilson7.github.io; make any 2D art with the TikZ method and the look of the other tools' title art; and put **privacy above everything else**: the tool must leak no information outside the browser, for any reason.

## The tool

Status Trends: a one-page static tool. The person exports issues from Jira (or any tracker, or a class gradebook) as a CSV file and opens it in the page. The page shows a dashboard: items per year, quarter or month split by status (still requested, pending, completed), the percentage change between periods and against the same period a year before, completion against a target, the open backlog, and a list of insights and risks worked out from the data. Everything is computed in the browser tab. Used in a desktop browser.

## Done means

- Opening a Jira CSV export (default Jira date format, and ISO dates) needs no editing of the file: the columns and status groups are guessed, and every guess can be changed in the page.
- Year, quarter and month views, with a fiscal year that can start in any month.
- Percentage change shown on the chart and in the table, against the previous period and the same period a year before.
- Insights and risks with a severity, an icon and a sentence each: completion below target, completion falling, backlog building, items open too long, items overdue, volume jumps, slower completion, groups below target.
- A teacher can use the same page with a class export (a sample shows how).
- Privacy, checked rather than claimed: no request of any kind after the page has loaded, no browser storage, no third-party anything, in three browser engines, while a file is opened and the dashboard is used.
- `./scripts/check.sh` passes.

## Decisions that stay with the person

- Whether the person's employer allows work data to be opened in a browser tool at all. The tool keeps data in the tab; it cannot answer a policy question.
- The default thresholds (target 80%, growth alert 20%, open-too-long 90 days).
- The sample data, which is invented.

## Autonomy

The plan is approved in advance: the person asked for the MVP to be built and deployed without stopping. The agent may plan, build, test, commit, create the public repository `sgilson7/status-trends`, publish its GitHub Pages site, and add the tool to sgilson7.github.io and deploy that site. It may never add a network request, a third-party script, font or service, a model, browser storage, analytics, or any route for data to leave the tab.

## Allowed core dependencies

`serde`, `serde_json` (reference default). No others.
