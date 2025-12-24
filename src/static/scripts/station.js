import { create, createSlider, createLoading, clear } from "./utils.js";

/*********/
/* model */
/*********/

export function initModel() {
  return {
    loading: false,
    open: true,
    map: null,
    ws: null,
    stations: null,
    currentStation: window.localStorage.getItem("current-station") || null,
  };
}

export const initialMsg = [
  {
    type: "StationMsg",
    data: { type: "CreateMap" },
  },
  {
    type: "StationMsg",
    data: { type: "Connect" },
  },
];

/**********/
/* update */
/**********/

export async function update(model, msg, dispatch) {
  console.log(msg, model);
  const globalDispatch = dispatch;
  dispatch = createDispatch(dispatch);
  switch (msg.type) {
    case "CheckEscape":
      return model;
    case "SelectSection":
      return { ...model, open: msg.data === "station" };
    case "CreateMap":
      createMap(dispatch);
      return { ...model, loading: true };
    case "CreatedMap":
      return { ...model, loading: false, map: msg.data };
    case "Connect":
      connect(dispatch, globalDispatch);
      return { ...model, loading: true };
    case "Connected":
      if (model.stations === null) {
        dispatch({ type: "GetStations" });
      }
      return { ...model, loading: false, ws: msg.data };
    case "Disconnected":
      setTimeout(() => dispatch({ type: "Connect" }), 3000);
      return { ...model, ws: null };
    case "GetStations":
      getStations(model.ws);
      return model;
    case "GotStations":
      return {
        ...model,
        stations: msg.data,
      };
    case "UpdateStation":
      updateStation(model.ws, msg.data);
      return model;
    case "UpdatedStation":
      window.localStorage.setItem("current-station", msg.data);
      document.getElementById("station-selection__station").value = "";
      document.getElementById("station-selection__n-years").value =
        document.getElementById("station-selection__n-years").min;
      dispatch({ type: "GetStations" });
      return { ...model, currentStation: msg.data };
    default:
      return model;
  }
}

function createDispatch(dispatch) {
  return (msg) => dispatch({ type: "StationMsg", data: msg });
}

async function createMap(dispatch) {
  const mapDiv = document.getElementById("main-station__map");
  const map = L.map(mapDiv);
  const resizeObserver = new ResizeObserver(() =>
    setTimeout(() => map.invalidateSize(), 300),
  );
  resizeObserver.observe(mapDiv);
  dispatch({ type: "CreatedMap", data: map });
}

function connect(dispatch, globalDispatch) {
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const ws = new WebSocket(`${protocol}//${window.location.host}/station/`);

  ws.onopen = () => {
    dispatch({ type: "Connected", data: ws });
  };
  ws.onmessage = (event) => {
    handleMessage(event, dispatch, globalDispatch);
  };
  ws.onclose = () => {
    dispatch({ type: "Disconnected" });
  };
  ws.onerror = (error) => {
    globalDispatch({
      type: "AddNotification",
      data: { text: `WebSocket error : ${error}`, isError: true },
    });
  };
}

function handleMessage(event, dispatch, globalDispatch) {
  const msg = JSON.parse(event.data);
  switch (msg.type) {
    case "error":
      globalDispatch({
        type: "AddNotification",
        data: { text: msg.data, isError: true },
      });
      break;
    case "stations":
      dispatch({ type: "GotStations", data: msg.data });
      break;
    case "station":
      dispatch({ type: "UpdatedStation", data: msg.data });
      break;
    default:
      break;
  }
}

function getStations(ws) {
  if (ws?.readyState === WebSocket.OPEN) {
    const data = {
      station: document.getElementById("station-selection__station").value,
      n_years: document.getElementById("station-selection__n-years").value,
    };
    ws.send(JSON.stringify({ type: "stations", data: data }));
  }
}

function updateStation(ws, station) {
  if (ws?.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ type: "station", data: station }));
  }
}

/********/
/* view */
/********/

export function initView(model, dispatch) {
  const globalDispatch = dispatch;
  dispatch = createDispatch(dispatch);
  initMetaView(model, globalDispatch);
  initMainView(model, dispatch);
}

export function view(model, dispatch) {
  dispatch = createDispatch(dispatch);

  openView(model);
  metaView(model);
  loadingView(model);
  autocompleteView(model, dispatch);

  if (model.map) {
    initMapView(model.map);
  }

  if (model.map && model.stations) {
    mapView(model, dispatch);
  }
}

function openView(model) {
  if (model.open) {
    document.getElementById("meta-station").classList.add("open");
    document.getElementById("main-station").classList.add("open");
  } else {
    document.getElementById("meta-station").classList.remove("open");
    document.getElementById("main-station").classList.remove("open");
  }
}

function loadingView(model) {
  if (model.loading) {
    document
      .querySelector("#main-station > .loading")
      .removeAttribute("hidden");
  } else {
    document
      .querySelector("#main-station > .loading")
      .setAttribute("hidden", true);
  }
}

function initMetaView(model, globalDispatch) {
  document.getElementById("meta").appendChild(
    create(
      "div",
      { id: "meta-station", class: model.open ? "open" : "" },
      [
        create("h2", {}, ["Station"]),
        create("span", { id: "station__station" }, []),
      ],
      [
        {
          event: "click",
          fct: () => {
            globalDispatch({ type: "SelectSection", data: "station" });
          },
        },
      ],
    ),
  );
}

function initMainView(model, dispatch) {
  document.getElementById("main").appendChild(
    create("section", { id: "main-station", class: model.open ? "open" : "" }, [
      create(
        "button",
        { class: "close" },
        [
          create("svg", { class: "icon" }, [
            create("use", { href: "#icon-x" }),
          ]),
        ],
        [
          {
            event: "click",
            fct: () => dispatch({ type: "SelectSection" }),
          },
        ],
      ),
      create("form", { id: "station-selection" }, [
        create("h2", {}, ["Sélection de la station hydrométrique"]),
        create("div", { class: "autocomplete" }, [
          create(
            "input",
            {
              id: "station-selection__station",
              placeholder: "Id ou nom de la station",
            },
            [],
            [{ event: "input", fct: () => dispatch({ type: "GetStations" }) }],
          ),
          create("div", { class: "autocomplete__list", hidden: true }),
        ]),
        create("label", {}, ["Nombre d'années disponibles"]),
        createSlider(
          "station-selection__n-years",
          10,
          50,
          true,
          [{ event: "change", fct: () => dispatch({ type: "GetStations" }) }],
          10,
        ),
      ]),
      create("div", { id: "main-station__map", hidden: true }),
      createLoading(),
    ]),
  );
}

function metaView(model) {
  if (model.currentStation !== null) {
    document.getElementById("station__station").textContent =
      model.currentStation;
  }
}

function initMapView(map) {
  if (Object.keys(map._layers).length == 0) {
    map.setView([47, -71], 6);
    L.tileLayer("/map/tile/{z}/{x}/{y}.png", {
      minZoom: 6,
      maxZoom: 12,
    }).addTo(map);
  }
  document.getElementById("main-station__map").removeAttribute("hidden");
}

function mapView(model, dispatch) {
  console.log(model.currentStation);
  d3.select(model.map.getPanes().overlayPane)
    .selectAll("div")
    .data(model.stations, (d) => d.station)
    .join(
      (enter) =>
        enter
          .append("div")
          .attr("class", (d) =>
            d.station === model.currentStation
              ? "map__marker map__marker--current"
              : "map__marker",
          )
          .attr("title", (d) => d.station)
          .style(
            "transform",
            (d) =>
              `translate(${model.map.latLngToLayerPoint([d.lat, d.lon]).x}px, ${model.map.latLngToLayerPoint([d.lat, d.lon]).y}px)`,
          )
          .style("width", (d) =>
            d.station === model.currentStation
              ? `${10 + (model.map._zoom - 6) * 2}px`
              : `${5 + (model.map._zoom - 6) * 2}px`,
          )
          .style("height", (d) =>
            d.station === model.currentStation
              ? `${10 + (model.map._zoom - 6) * 2}px`
              : `${5 + (model.map._zoom - 6) * 2}px`,
          )
          .on("click", (event) => {
            dispatch({
              type: "UpdateStation",
              data: event.target.__data__.station,
            });
          }),
      (update) =>
        update
          .attr("class", (d) =>
            d.station === model.currentStation
              ? "map__marker map__marker--current"
              : "map__marker",
          )
          .style("width", (d) =>
            d.station === model.currentStation
              ? `${10 + (model.map._zoom - 6) * 2}px`
              : `${5 + (model.map._zoom - 6) * 2}px`,
          )
          .style("height", (d) =>
            d.station === model.currentStation
              ? `${10 + (model.map._zoom - 6) * 2}px`
              : `${5 + (model.map._zoom - 6) * 2}px`,
          ),
    );

  function updateLocation() {
    d3.selectAll(".map__marker")
      .style(
        "transform",
        (d) =>
          `translate(${model.map.latLngToLayerPoint([d.lat, d.lon]).x}px, ${model.map.latLngToLayerPoint([d.lat, d.lon]).y}px)`,
      )
      .style("width", (d) =>
        d.station === model.currentStation
          ? `${10 + (model.map._zoom - 6) * 2}px`
          : `${5 + (model.map._zoom - 6) * 2}px`,
      )
      .style("height", (d) =>
        d.station === model.currentStation
          ? `${10 + (model.map._zoom - 6) * 2}px`
          : `${5 + (model.map._zoom - 6) * 2}px`,
      );
  }

  model.map.on("moveend", updateLocation);
}

function autocompleteView(model, dispatch) {
  const input = document.getElementById("station-selection__station");
  const div = document.querySelector("#station-selection .autocomplete__list");
  clear(div);
  if (input.value !== "" && model.stations !== null) {
    div.removeAttribute("hidden");
    model.stations.forEach((station) => {
      div.appendChild(
        create(
          "span",
          {},
          [station.station],
          [
            {
              event: "click",
              fct: () => {
                dispatch({ type: "UpdateStation", data: station.station });
              },
            },
          ],
        ),
      );
    });
  } else {
    div.setAttribute("hidden", true);
  }
}
