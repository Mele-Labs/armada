// The time format in `@armada/components`: how an instant is written under each of the three switches,
// how long ago reads, and the text Fleet keeps the format as. In node: nothing here needs a document.
//
// **The instant is built in local time**, so the clock it shows is 14:02:09 on any machine's zone. The
// locale is the host's, as the shipped code leaves it (`duration.test.ts` takes the same stance), so the
// spellings below are English ones; a space is any space, because newer ICU writes "2:02 PM" with a
// narrow no-break one.

import { afterEach, describe, expect, it } from "vitest";
import {
  DEFAULT_TIME_FORMAT,
  ageOf,
  parseTimeFormat,
  saveTimeFormat,
  setTimeFormat,
  setTimeFormatSaver,
  stampOf,
  timeFormat,
  timeSaid,
  type TimeFormat,
} from "@armada/components";

afterEach(() => {
  setTimeFormatSaver(null);
  setTimeFormat(DEFAULT_TIME_FORMAT);
});

const plain = (text: string): string => text.replace(/\s/g, " ");

const AFTERNOON = new Date(2026, 9, 19, 14, 2, 9).getTime();
const MORNING = new Date(2026, 9, 19, 9, 5, 7).getTime();
const YESTERDAY = new Date(2026, 9, 18, 14, 2, 9).getTime();
const LAST_MONTH = new Date(2026, 8, 30, 23, 59, 0).getTime();
const TOMORROW_EARLY = new Date(2026, 9, 20, 0, 0, 30).getTime();

describe("a stamp under each of the three switches", () => {
  const cases: [TimeFormat, string][] = [
    [{ hour12: false, seconds: false, date: false }, "14:02"],
    [{ hour12: false, seconds: true, date: false }, "14:02:09"],
    [{ hour12: true, seconds: false, date: false }, "2:02 PM"],
    [{ hour12: true, seconds: true, date: false }, "2:02:09 PM"],
    [{ hour12: false, seconds: false, date: true }, "Oct 19 14:02"],
    [{ hour12: false, seconds: true, date: true }, "Oct 19 14:02:09"],
    [{ hour12: true, seconds: false, date: true }, "Oct 19 2:02 PM"],
    [{ hour12: true, seconds: true, date: true }, "Oct 19 2:02:09 PM"],
  ];

  it.each(cases)("%j reads %s", (format, written) => {
    expect(plain(stampOf(AFTERNOON, format, AFTERNOON))).toBe(written);
  });

  it("pads the hour on a 24-hour clock and not on a 12-hour one", () => {
    expect(plain(stampOf(MORNING, { hour12: false, seconds: true, date: false }, AFTERNOON))).toBe("09:05:07");
    expect(plain(stampOf(MORNING, { hour12: true, seconds: true, date: false }, AFTERNOON))).toBe("9:05:07 AM");
  });
});

describe("a stamp from an earlier day, with the date switch off", () => {
  const OFF: TimeFormat = { hour12: false, seconds: false, date: false };

  it("keeps its day, so a time is never read as today's", () => {
    expect(plain(stampOf(YESTERDAY, OFF, AFTERNOON))).toBe("Oct 18 14:02");
    expect(plain(stampOf(LAST_MONTH, OFF, AFTERNOON))).toBe("Sep 30 23:59");
  });

  it("follows the other two switches", () => {
    expect(plain(stampOf(YESTERDAY, { hour12: true, seconds: true, date: false }, AFTERNOON))).toBe("Oct 18 2:02:09 PM");
  });

  it("is a time alone when it is from today, even just after midnight", () => {
    expect(plain(stampOf(MORNING, OFF, AFTERNOON))).toBe("09:05");
    expect(plain(stampOf(TOMORROW_EARLY, OFF, TOMORROW_EARLY + 60_000))).toBe("00:00");
  });

  it("is from yesterday by the calendar, not by 24 hours", () => {
    expect(plain(stampOf(LAST_MONTH, OFF, new Date(2026, 9, 1, 0, 5, 0).getTime()))).toBe("Sep 30 23:59");
    expect(plain(stampOf(LAST_MONTH, OFF, new Date(2026, 8, 30, 23, 59, 30).getTime()))).toBe("23:59");
  });

  it("is dated the same with the date switch on", () => {
    expect(plain(stampOf(YESTERDAY, { ...OFF, date: true }, AFTERNOON))).toBe("Oct 18 14:02");
    expect(plain(stampOf(MORNING, { ...OFF, date: true }, AFTERNOON))).toBe("Oct 19 09:05");
  });

  it("reads the same through the plain-text path", () => {
    expect(plain(timeSaid(YESTERDAY, OFF, AFTERNOON))).toBe("Oct 18 14:02");
    expect(plain(timeSaid(new Date(AFTERNOON).toISOString(), OFF, AFTERNOON))).toBe("14:02");
  });
});

describe("how long ago, to one unit", () => {
  const NOW = new Date(2026, 9, 19, 14, 0, 0).getTime();
  const MINUTE = 60_000;
  const HOUR = 60 * MINUTE;
  const DAY = 24 * HOUR;

  it("is under a minute until the minute is whole", () => {
    expect(ageOf(NOW, NOW)).toBe("<1m ago");
    expect(ageOf(NOW - (MINUTE - 1), NOW)).toBe("<1m ago");
  });

  it("counts whole minutes up to the hour", () => {
    expect(ageOf(NOW - MINUTE, NOW)).toBe("1m ago");
    expect(ageOf(NOW - 4 * MINUTE - 30_000, NOW)).toBe("4m ago");
    expect(ageOf(NOW - (HOUR - 1), NOW)).toBe("59m ago");
  });

  it("counts whole hours up to the day", () => {
    expect(ageOf(NOW - HOUR, NOW)).toBe("1h ago");
    expect(ageOf(NOW - 2 * HOUR - 59 * MINUTE, NOW)).toBe("2h ago");
    expect(ageOf(NOW - (DAY - 1), NOW)).toBe("23h ago");
  });

  it("counts whole days from the first", () => {
    expect(ageOf(NOW - DAY, NOW)).toBe("1d ago");
    expect(ageOf(NOW - 3 * DAY - 5 * HOUR, NOW)).toBe("3d ago");
  });

  it("reads an instant ahead of now as the first, never a negative", () => {
    expect(ageOf(NOW + 5 * MINUTE, NOW)).toBe("<1m ago");
  });
});

describe("the text Fleet keeps the format as", () => {
  const SET: TimeFormat = { hour12: true, seconds: false, date: true };

  it("gives back the format the text spells", () => {
    expect(parseTimeFormat(JSON.stringify(SET))).toEqual(SET);
  });

  it("gives the shipped default for text that is not JSON", () => {
    expect(parseTimeFormat("{not json")).toEqual(DEFAULT_TIME_FORMAT);
    expect(parseTimeFormat("")).toEqual(DEFAULT_TIME_FORMAT);
  });

  it("gives the shipped default, not half of another, when a field is missing", () => {
    expect(parseTimeFormat('{"hour12":true,"seconds":false}')).toEqual(DEFAULT_TIME_FORMAT);
    expect(parseTimeFormat('{"hour12":true,"date":true}')).toEqual(DEFAULT_TIME_FORMAT);
  });

  it("gives the shipped default for a field that is not a boolean, or JSON that is not an object", () => {
    expect(parseTimeFormat('{"hour12":"yes","seconds":false,"date":true}')).toEqual(DEFAULT_TIME_FORMAT);
    expect(parseTimeFormat("null")).toEqual(DEFAULT_TIME_FORMAT);
    expect(parseTimeFormat("42")).toEqual(DEFAULT_TIME_FORMAT);
    expect(parseTimeFormat("[true,true,true]")).toEqual(DEFAULT_TIME_FORMAT);
  });
});

describe("a change the person made", () => {
  const CHANGED: TimeFormat = { hour12: true, seconds: false, date: true };

  it("is sent to the saver as JSON text, and read back as the format now set", () => {
    const sent: string[] = [];
    setTimeFormatSaver((text) => sent.push(text));

    saveTimeFormat(CHANGED);

    expect(sent).toHaveLength(1);
    expect(JSON.parse(sent[0]!)).toEqual(CHANGED);
    expect(parseTimeFormat(sent[0]!)).toEqual(CHANGED);
    expect(timeFormat()).toEqual(CHANGED);
  });

  it("is drawn now even where nothing is set to keep it", () => {
    setTimeFormatSaver(null);

    saveTimeFormat(CHANGED);

    expect(timeFormat()).toEqual(CHANGED);
  });

  it("is not sent when it is Fleet's own answer coming back", () => {
    const sent: string[] = [];
    setTimeFormatSaver((text) => sent.push(text));

    setTimeFormat(CHANGED);

    expect(timeFormat()).toEqual(CHANGED);
    expect(sent).toEqual([]);
  });
});
