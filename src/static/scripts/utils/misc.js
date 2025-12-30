export const frenchLocale = d3.timeFormatLocale({
  dateTime: "%A %e %B %Y à %X",
  date: "%Y-%m-%d",
  time: "%H:%M:%S",
  periods: ["", ""],
  days: [
    "dimanche",
    "lundi",
    "mardi",
    "mercredi",
    "jeudi",
    "vendredi",
    "samedi",
  ],
  shortDays: ["dim", "lun", "mar", "mer", "jeu", "ven", "sam"],
  months: [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
  ],
  shortMonths: [
    "jan",
    "fév",
    "mar",
    "avr",
    "mai",
    "jun",
    "jul",
    "aoû",
    "sep",
    "oct",
    "nov",
    "déc",
  ],
});

export function round(n, d) {
  return Math.round(n * 10 ** d) / 10 ** d;
}

export function formatNumber(n) {
  return n.toLocaleString("en-US").replace(/,/g, " ");
}

export function range(start, end) {
  if (end === undefined) {
    return [...Array(start).keys()];
  } else {
    return [...Array(end).keys()].filter((x) => x >= start);
  }
}

export function toTitle(text) {
  return text
    .split()
    .map((w) => w[0].toUpperCase() + w.slice(1))
    .join(" ");
}
