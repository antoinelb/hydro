import { create, clear } from "./utils/elements.js";
import { connect } from "./utils/ws.js";
import { toTitle, formatNumber, frenchLocale } from "./utils/misc.js";

/*********/
/* model */
/*********/

export function initModel() {
  return {
    loading: false,
    open: false,
    ws: null,
    station: null,
    petModels: null,
    petModel: window.localStorage.getItem("pet_model") || "oudin",
    nValYears: parseInt(window.localStorage.getItem("n_val_years") || "5"),
    hydroData: null,
    weatherData: null,
    precipitationData: null,
    calibData: null,
    validData: null,
  };
}

export const initialMsg = [
  {
    type: "DataMsg",
    data: { type: "Connect" },
  },
];

/**********/
/* update */
/**********/

export async function update(model, msg, dispatch, createNotification) {
  dispatch = createDispatch(dispatch);
  switch (msg.type) {
    case "CheckEscape":
      return model;
    case "SelectSection":
      const open = msg.data === "data";
      return { ...model, open: open };
    case "UpdateStation":
      return { ...model, station: msg.data };
    case "Connect":
      connect("data/", handleMessage, dispatch, createNotification);
      return { ...model, loading: true };
    case "Connected":
      if (model.petModels === null) {
        dispatch({ type: "GetModels" });
      }
      if (model.station !== null && model.hydroData === null) {
        dispatch({
          type: "GetData",
          data: { station: model.station, type: "hydro" },
        });
      }
      return { ...model, loading: false, ws: msg.data };
    case "Disconnected":
      setTimeout(() => dispatch({ type: "Connect" }), 3000);
      return { ...model, ws: null };
    case "GetModels":
      if (model.ws?.readyState === WebSocket.OPEN) {
        model.ws.send(JSON.stringify({ type: "models" }));
      }
      return { ...model, loading: true };
    case "GotModels":
      return {
        ...model,
        loading: false,
        petModels: msg.data.pet,
      };
    case "UpdateModel":
      if (model.ws?.readyState === WebSocket.OPEN) {
        model.ws.send(
          JSON.stringify({
            type: "model",
            data: { type: msg.data.type, val: msg.data.val },
          }),
        );
      }
      return model;
    case "UpdatedModel":
      switch (msg.data.type) {
        case "pet":
          window.localStorage.setItem("pet_model", msg.data.val);
          model = { ...model, petModel: msg.data.val };
          getDatasets(model, dispatch);
          return model;
        default:
          return model;
      }
    case "UpdateValidationYears":
      if (model.ws?.readyState === WebSocket.OPEN) {
        model.ws.send(
          JSON.stringify({
            type: "validation_years",
            data: parseInt(msg.data),
          }),
        );
      }
      return model;
    case "UpdatedValidationYears":
      window.localStorage.setItem("n_val_years", msg.data.toString());
      model = { ...model, nValYears: msg.data };
      getDatasets(model, dispatch);
      return model;
    case "GetData":
      if (model.ws?.readyState === WebSocket.OPEN) {
        model.ws.send(
          JSON.stringify({
            type: `${msg.data.type}_data`,
            data: { station: msg.data.station },
          }),
        );
      }
      return { ...model, loading: true };
    case "GotHydroData":
      dispatch({
        type: "GetData",
        data: { station: model.station, type: "weather" },
      });
      return { ...model, loading: false, hydroData: msg.data };
    case "GotWeatherData":
      dispatch({
        type: "GetData",
        data: { station: model.station, type: "precipitation" },
      });
      return { ...model, loading: false, weatherData: msg.data };
    case "GotPrecipitationData":
      model = { ...model, loading: false, precipitationData: msg.data };
      getDatasets(model, dispatch);
      return model;
    case "GetDatasets":
      if (model.ws?.readyState === WebSocket.OPEN) {
        model.ws.send(
          JSON.stringify({
            type: "datasets",
            data: {
              station: model.station,
              pet_model: model.petModel,
              n_valid_years: model.nValYears,
            },
          }),
        );
      }
      return { ...model, loading: true };
    case "GotDatasets":
      return {
        ...model,
        loading: false,
        calibData: msg.data.calibration,
        validData: msg.data.validation,
      };
    default:
      return model;
  }
}

function createDispatch(dispatch) {
  return (msg) => dispatch({ type: "DataMsg", data: msg });
}

function handleMessage(event, dispatch, createNotification) {
  const msg = JSON.parse(event.data);
  switch (msg.type) {
    case "error":
      createNotification(msg.data, true);
      break;
    case "models":
      dispatch({ type: "GotModels", data: msg.data });
      break;
    case "model":
      dispatch({ type: "UpdatedModel", data: msg.data });
      break;
    case "validation_years":
      dispatch({ type: "UpdatedValidationYears", data: msg.data });
      break;
    case "hydro_data":
      dispatch({ type: "GotHydroData", data: msg.data });
      break;
    case "weather_data":
      dispatch({ type: "GotWeatherData", data: msg.data });
      break;
    case "precipitation_data":
      dispatch({ type: "GotPrecipitationData", data: msg.data });
      break;
    case "datasets":
      dispatch({ type: "GotDatasets", data: msg.data });
      break;
    default:
      createNotification("Unknown websocket message", true);
      break;
  }
}

function getDatasets(model, dispatch) {
  if (
    model.hydroData !== null &&
    model.weatherData !== null &&
    model.precipitationData !== null &&
    model.petModel !== null &&
    model.nValYears !== null
  ) {
    dispatch({ type: "GetDatasets" });
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
  formView(model);
  statsView(model);
  dataView(model.hydroData, "discharge", false, true);
  dataView(model.weatherData, "temperature", false, false);
  dataView(model.precipitationData, "precipitation", true, false);
}

function openView(model) {
  if (model.open) {
    document.getElementById("data-meta").classList.add("open");
    document.getElementById("data-main").classList.add("open");
  } else {
    document.getElementById("data-meta").classList.remove("open");
    document.getElementById("data-main").classList.remove("open");
  }
}

function initMetaView(model, globalDispatch) {
  document.getElementById("meta").appendChild(
    create(
      "div",
      { id: "data-meta", class: model.open ? "open" : "" },
      [
        create("h2", {}, ["Données"]),
        create(
          "span",
          { class: "data__pet" },
          "Modèle d'évapotranspiration potentielle:",
        ),
        create("span", { id: "data__pet", class: "data__pet" }, []),
        create("span", {}, "Nombre d'années de validation:"),
        create("span", { id: "data__val-years" }, []),
      ],
      [
        {
          event: "click",
          fct: () => {
            globalDispatch({ type: "SelectSection", data: "data" });
          },
        },
      ],
    ),
  );
}

function initMainView(model, dispatch) {
  document.getElementById("main").appendChild(
    create("section", { id: "data-main", class: model.open ? "open" : "" }, [
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
      create(
        "form",
        {},
        [
          create("label", { for: "data-selection__pet" }, [
            "Évapotranspiration potentielle:",
            create(
              "select",
              { id: "data-selection__pet", hidden: true },
              [],
              [
                {
                  event: "input",
                  fct: (event) => {
                    dispatch({
                      type: "UpdateModel",
                      data: { type: "pet", val: event.target.value },
                    });
                  },
                },
              ],
            ),
          ]),
          create("label", { for: "data-selection__val-years" }, [
            "Années validation:",
            create(
              "input",
              {
                id: "data-selection__val-years",
                type: "number",
                value: model.nValYears.toString(),
                min: "0",
              },
              [],
              [
                {
                  event: "input",
                  fct: (event) => {
                    dispatch({
                      type: "UpdateValidationYears",
                      data: event.target.value,
                    });
                  },
                },
              ],
            ),
          ]),
        ],
        [{ event: "submit", fct: (event) => event.preventDefault() }],
      ),
      create("div", { id: "data-main__stats" }, [
        create("span", { class: "label" }, ["# données hydrologiques"]),
        create("span", { class: "label" }, ["# données météo"]),
        create("span", { class: "label" }, ["# données précipitation"]),
        create("span", { id: "data-main__stats__hydro" }, []),
        create("span", { id: "data-main__stats__weather" }, []),
        create("span", { id: "data-main__stats__precip" }, []),
        create("span", { class: "label" }, ["# données calibration"]),
        create("span", { class: "label" }, ["# données validation"]),
        create("span", { id: "data-main__stats__calib" }, []),
        create("span", { id: "data-main__stats__valid" }, []),
      ]),
      create("div", { id: "data-main__plots" }, [
        create("svg", { id: "data-main__discharge", class: "plot" }),
        create("svg", { id: "data-main__temperature", class: "plot" }),
        create("svg", { id: "data-main__precipitation", class: "plot" }),
      ]),
    ]),
  );
}

function metaView(model) {
  if (model.petModel === null) {
    document.getElementById("data__pet").textContent = "";
    [...document.querySelectorAll(".data__pet")].forEach((span) =>
      span.classList.add("disabled"),
    );
  } else {
    document.getElementById("data__pet").textContent = toTitle(model.petModel);
    [...document.querySelectorAll(".data__pet")].forEach((span) =>
      span.classList.remove("disabled"),
    );
  }

  document.getElementById("data__val-years").textContent = model.nValYears;
}

function formView(model) {
  const petSelect = document.getElementById("data-selection__pet");

  if (petSelect.children.length === 0 && model.petModels !== null) {
    petSelect.removeAttribute("hidden");
    model.petModels.forEach((_model) => {
      const option = create("option", { value: _model }, [toTitle(_model)]);
      option.selected = _model === model.petModel;
      petSelect.appendChild(option);
    });
  }
}

function dataView(data, feature, showXLabels = false, showLegend = false) {
  const translateFeature = {
    discharge: "débit",
    temperature: "température",
    precipitation: "précipitation",
  };
  const _svg = document.getElementById(`data-main__${feature}`);
  clear(_svg);
  if (data === null) {
    _svg.setAttribute("hidden", true);
  } else {
    _svg.removeAttribute("hidden");
    const width = _svg.clientWidth;
    const height = _svg.clientHeight;
    _svg.setAttribute("viewBox", `0 0 ${width} ${height}`);

    const boundaries = {
      l: 50,
      r: width - 25,
      t: 5,
      b: height - (showXLabels ? 50 : 10),
    };

    const svg = d3.select(_svg);

    const xScale = d3
      .scaleTime()
      .domain(d3.extent(data, (d) => new Date(d.date)))
      .range([boundaries.l, boundaries.r]);
    const yScale = d3
      .scaleLinear()
      .domain([
        d3.min(data, (d) => d[`${feature}_min`]),
        d3.max(data, (d) => d[`${feature}_max`]),
      ])
      .range([boundaries.b, boundaries.t]);

    // x axis
    const xAxis = svg
      .append("g")
      .attr("class", "x-axis")
      .attr("transform", `translate(0, ${boundaries.b})`)
      .call(d3.axisBottom(xScale).tickFormat(frenchLocale.format("%B")));
    if (showXLabels) {
      xAxis
        .selectAll("text")
        .attr("transform", "rotate(-45)")
        .attr("text-anchor", "end")
        .attr("dx", "-0.5em")
        .attr("dy", "0.5em");
    } else {
      xAxis.selectAll("text").remove();
    }
    // y axis
    svg
      .append("g")
      .attr("class", "y-axis")
      .attr("transform", `translate(${boundaries.l}, 0)`)
      .call(
        d3
          .axisLeft(yScale)
          .ticks(5)
          .tickFormat((x) => formatNumber(x)),
      );
    svg
      .append("text")
      .attr("x", 15)
      .attr("y", (boundaries.t + boundaries.b) / 2)
      .attr("text-anchor", "middle")
      .attr("dominant-baseline", "middle")
      .attr(
        "transform",
        `rotate(-90, 15, ${(boundaries.t + boundaries.b) / 2})`,
      )
      .attr("font-size", "0.9rem")
      .text(toTitle(translateFeature[feature]));

    // current data
    svg
      .append("path")
      .attr("class", "path-red")
      .datum(data)
      .attr(
        "d",
        d3
          .line()
          .x((d) => xScale(new Date(d.date)))
          .y((d) => yScale(d[feature])),
      );

    // historical median and min-max range
    svg
      .append("path")
      .datum(data)
      .attr(
        "d",
        d3
          .line()
          .x((d) => xScale(new Date(d.date)))
          .y((d) => yScale(d[`${feature}_median`])),
      );
    svg
      .append("path")
      .datum(data)
      .attr("class", "path-area")
      .attr(
        "d",
        d3
          .area()
          .x((d) => xScale(new Date(d.date)))
          .y0((d) => yScale(d[`${feature}_min`]))
          .y1((d) => yScale(d[`${feature}_max`])),
      );

    // legend
    if (showLegend) {
      const legendData = [
        { label: "Dernière année", class: "path-red", type: "line" },
        { label: "Médiane historique", class: "", type: "line" },
        { label: "Écart min-max historique", class: "", type: "area" },
      ];
      const legend = svg
        .append("g")
        .attr("class", "legend")
        .attr("transform", `translate(${boundaries.r - 150}, ${boundaries.t})`);
      const legendItems = legend
        .selectAll(".legend-item")
        .data(legendData)
        .enter()
        .append("g")
        .attr("class", "legend-item")
        .attr("transform", (_, i) => `translate(0, ${i * 20})`);
      legendItems
        .filter((d) => d.type === "line")
        .append("line")
        .attr("class", (d) => d.class)
        .attr("x1", 0)
        .attr("x2", 15)
        .attr("y1", 7)
        .attr("y2", 7);
      legendItems
        .filter((d) => d.type === "area")
        .append("rect")
        .attr("class", (d) => d.class)
        .attr("width", 15)
        .attr("height", 15);
      legendItems
        .append("text")
        .attr("x", 20)
        .attr("y", 12)
        .text((d) => d.label);
    }
  }
}

function statsView(model) {
  if (model.hydroData !== null) {
    document.getElementById("data-main__stats__hydro").textContent =
      formatNumber(model.hydroData.length);
  }
  if (model.weatherData !== null) {
    document.getElementById("data-main__stats__weather").textContent =
      formatNumber(model.weatherData.length);
  }
  if (model.precipitationData !== null) {
    document.getElementById("data-main__stats__precip").textContent =
      formatNumber(model.precipitationData.length);
  }
  if (model.calibData !== null) {
    document.getElementById("data-main__stats__calib").textContent =
      formatNumber(model.calibData.length);
  }
  if (model.validData !== null) {
    document.getElementById("data-main__stats__valid").textContent =
      formatNumber(model.validData.length);
  }
}
