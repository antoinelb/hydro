/**
 * Leaflet wrapper module
 * Re-exports the global L object loaded via script tag for use with ES6 imports
 * This provides intellisense and explicit imports while keeping fast preloading
 */

if (typeof window.L === 'undefined') {
  console.error('Leaflet not loaded! Make sure leaflet.min.js is loaded via script tag before this module.');
}

export default window.L;
export const {
  map,
  tileLayer,
  marker,
  circle,
  polygon,
  polyline,
  rectangle,
  icon,
  divIcon,
  latLng,
  latLngBounds,
  point,
  bounds,
  control,
  geoJSON,
  popup,
  tooltip,
  layerGroup,
  featureGroup,
  // Add other commonly used Leaflet exports as needed
} = window.L || {};
