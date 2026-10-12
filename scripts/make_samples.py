#!/usr/bin/env python3
"""Write the two invented sample files in data/. Every value is made up.

    python3 scripts/make_samples.py

The generator is seeded, so it writes the same files every time. The
samples exist to show the tool, not to stand for any real team or class:

- data/sample-requests.csv: a Jira-style export of team requests from
  January 2023 to October 2026, in Jira's default date format. Volume grows
  each year; in 2026 one team falls behind, so the dashboard has risks to find.
- data/sample-class.csv: one row per student per assignment for a program
  running from August 2025, with ISO dates. A few students fall behind.
"""
import csv
import datetime as dt
import pathlib
import random

DATA = pathlib.Path(__file__).resolve().parent.parent / "data"
MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]


def jira(t):
    hour = t.hour % 12 or 12
    return f"{t.day:02d}/{MONTHS[t.month - 1]}/{t.year % 100:02d} {hour}:{t.minute:02d} {'AM' if t.hour < 12 else 'PM'}"


def requests():
    rng = random.Random(20261012)
    start = dt.datetime(2023, 1, 3, 9, 0)
    end = dt.datetime(2026, 10, 9, 17, 0)
    teams = ["Platform", "Data", "Design", "Support"]
    kinds = ["Task", "Story", "Bug"]
    topics = ["access request", "report change", "dashboard update", "data pull", "new field",
              "permissions review", "export fix", "workflow change", "integration check", "page copy"]
    rows = []
    day = start
    key = 1
    while day < end:
        years = (day - start).days / 365.0
        # About 1.2 a working day in 2023, rising about 22% a year.
        rate = 1.2 * (1.22 ** years)
        if day.weekday() < 5:
            for _ in range(int(rate) + (1 if rng.random() < rate - int(rate) else 0)):
                team = rng.choices(teams, weights=[3, 3, 2, 2])[0]
                created = day + dt.timedelta(hours=rng.randint(0, 7), minutes=rng.randint(0, 59))
                age = (end - created).days
                # Time to complete: Data slows down in 2026.
                slow = team == "Data" and created.year == 2026
                days = int(rng.expovariate(1 / (45 if slow else 15))) + 1
                p_done = 0.93 if not slow else 0.55
                roll = rng.random()
                if roll < 0.04:
                    status = rng.choice(["Won't Do", "Duplicate"])
                    resolved = created + dt.timedelta(days=rng.randint(1, 10))
                elif roll < 0.04 + p_done and days < age:
                    status = "Done"
                    resolved = created + dt.timedelta(days=days, hours=rng.randint(0, 6))
                else:
                    status = rng.choices(["To Do", "In Progress", "In Review", "Blocked"], weights=[4, 3, 2, 1])[0]
                    resolved = None
                due = created + dt.timedelta(days=rng.randint(10, 45)) if rng.random() < 0.45 else None
                updated = resolved or min(end, created + dt.timedelta(days=rng.randint(0, max(age, 0))))
                rows.append({
                    "Summary": f"{team} {rng.choice(topics)}",
                    "Issue key": f"REQ-{key}",
                    "Issue Type": rng.choices(kinds, weights=[5, 3, 2])[0],
                    "Status": status,
                    "Priority": rng.choices(["Low", "Medium", "High"], weights=[3, 5, 2])[0],
                    "Team": team,
                    "Created": jira(created),
                    "Updated": jira(updated),
                    "Resolved": jira(resolved) if resolved else "",
                    "Due date": jira(due.replace(hour=0, minute=0)) if due else "",
                })
                key += 1
        day += dt.timedelta(days=1)
    with open(DATA / "sample-requests.csv", "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    return len(rows)


def students():
    rng = random.Random(20250818)
    names = [f"Student {i:02d}" for i in range(1, 25)]
    # Three students fall behind as the year goes on.
    behind = {"Student 07", "Student 15", "Student 22"}
    units = ["Unit 1", "Unit 2", "Unit 3", "Unit 4", "Unit 5", "Unit 6", "Unit 7", "Unit 8"]
    today = dt.date(2026, 10, 9)
    rows = []
    week = dt.date(2025, 8, 18)
    n = 1
    breaks = [(dt.date(2025, 12, 20), dt.date(2026, 1, 5)), (dt.date(2026, 6, 6), dt.date(2026, 8, 17))]
    while week <= today:
        if not any(a <= week <= b for a, b in breaks):
            unit = units[min(len(units) - 1, ((week - dt.date(2025, 8, 18)).days // 49) % len(units))]
            for k in range(2):
                assigned = week + dt.timedelta(days=1 + 2 * k)
                if assigned > today:
                    continue
                due = assigned + dt.timedelta(days=7)
                title = f"Assignment {n}"
                n += 1
                months_in = (assigned - dt.date(2025, 8, 18)).days / 30
                for s in names:
                    if rng.random() < 0.02:
                        status, turned = "Excused", ""
                    else:
                        p = 0.94 if s not in behind else max(0.35, 0.9 - 0.05 * months_in)
                        late = rng.randint(-5, 3) if s not in behind else rng.randint(-2, 9)
                        when = due + dt.timedelta(days=late)
                        if rng.random() < p and when <= today:
                            status = rng.choices(["Graded", "Turned in"], weights=[4, 1])[0] if when < today - dt.timedelta(days=5) else "Turned in"
                            turned = when.isoformat()
                        elif due < today:
                            status, turned = rng.choices(["Missing", "In progress"], weights=[3, 1])[0], ""
                        else:
                            status, turned = rng.choices(["Not started", "In progress"], weights=[2, 1])[0], ""
                    rows.append({
                        "Student": s, "Unit": unit, "Assignment": title,
                        "Assigned": assigned.isoformat(), "Due": due.isoformat(),
                        "Status": status, "Turned in": turned,
                    })
        week += dt.timedelta(days=7)
    with open(DATA / "sample-class.csv", "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    return len(rows)


if __name__ == "__main__":
    print(f"data/sample-requests.csv: {requests()} rows")
    print(f"data/sample-class.csv: {students()} rows")
