import { create, createSlider, createLoading } from "./utils.js";

/*********/
/* model */
/*********/

export function initModel() {
  return {
    loading: false,
    open: true,
    map: null,
  };
}

export const initialMsg = {
  type: "StationMsg",
  data: { type: "CreateMap" },
};

/**********/
/* update */
/**********/

export async function update(model, msg, dispatch) {
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
  loadingView(model);

  if (model.map) {
    initMapView(model.map);
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
    document.getElementById("main-station__map").setAttribute("hidden", true);
  } else {
    document
      .querySelector("#main-station > .loading")
      .setAttribute("hidden", true);
    document.getElementById("main-station__map").removeAttribute("hidden");
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
        create("input", {
          id: "station-selection__station",
          placeholder: "Id ou nom de la station",
        }),
        create("label", {}, ["Nombre d'années disponibles"]),
        createSlider("station-selection__n-years", 10, 50, true),
      ]),
      create("div", { id: "main-station__map", hidden: true }),
      createLoading(),
    ]),
  );
}
