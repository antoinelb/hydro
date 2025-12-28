/**
 * D3.js wrapper module
 * Re-exports the global d3 object loaded via script tag for use with ES6 imports
 * This provides intellisense and explicit imports while keeping fast preloading
 */

if (typeof window.d3 === "undefined") {
  console.error(
    "D3.js not loaded! Make sure d3.v7.min.js is loaded via script tag before this module.",
  );
}

export default window.d3;
export const {
  area,
  axisBottom,
  axisLeft,
  axisRight,
  axisTop,
  drag,
  extent,
  line,
  min,
  max,
  scaleLinear,
  scaleOrdinal,
  scaleTime,
  schemeCategory10,
  select,
  selectAll,
  selection,
  timeMonth,
  timeYear,
  zoom,
  zoomIdentity,
} = window.d3 || {};
