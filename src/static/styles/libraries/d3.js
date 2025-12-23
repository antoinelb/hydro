/**
 * D3.js wrapper module
 * Re-exports the global d3 object loaded via script tag for use with ES6 imports
 * This provides intellisense and explicit imports while keeping fast preloading
 */

if (typeof window.d3 === 'undefined') {
  console.error('D3.js not loaded! Make sure d3.v7.min.js is loaded via script tag before this module.');
}

export default window.d3;
export const {
  select,
  selectAll,
  selection,
  drag,
  zoom,
  zoomIdentity,
  forceSimulation,
  forceLink,
  forceManyBody,
  forceCenter,
  forceCollide,
  forceX,
  forceY,
  scaleLinear,
  scaleOrdinal,
  schemeCategory10,
  // Add other commonly used d3 exports as needed
} = window.d3 || {};
