// The one search bar, mock only: System, Projects, Reviews and Issues each narrow their list by typing, with / reaching it and Esc
// clearing it. Over the scenario `search-bars`.

import { button, inside, place, region, role, systemPage, walk } from "../walk";
import type { Step } from "../walk";

const systemSearch = role("searchbox", "Search System");
const taskSearch = role("searchbox", "Search the tasks");
const pullSearch = role("searchbox", "Search the pull requests");
const issueSearch = role("searchbox", "Search the issues");

function steps(): Step[] {
  return [
    { press: place("System"), say: "System" },
    { type: "serv", into: systemSearch, say: "Words" },
    { look: systemPage("Servers"), say: "The entries that match" },
    { key: "Escape", on: systemSearch, say: "Esc clears the words" },
    { look: systemPage("Skills"), say: "Every entry is back" },
    { press: place("Projects"), say: "Projects" },
    { key: "/", on: region("Projects", { exact: true }), say: "/ goes to the search" },
    { type: "pairing", into: taskSearch, say: "Words" },
    { look: region("Search results"), say: "Every task that matches" },
    { key: "Escape", on: taskSearch, say: "Esc clears the words" },
    { press: place("Reviews"), say: "Reviews" },
    { type: "marta", into: pullSearch, say: "An author" },
    { look: inside(region("Pull requests"), button(/^#1899 /)), say: "The pull requests by that author" },
    { press: place("Issues"), say: "Issues" },
    { type: "socket", into: issueSearch, say: "A word from a title" },
    { look: inside(region("Issues", { exact: true }), button(/^#2801 /)), say: "The issues that match, in every repository" },
  ];
}

const searchBars = walk("search-bars", steps());

export { searchBars as "search-bars" };
