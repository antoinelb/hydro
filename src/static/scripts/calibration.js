import { create, clear, createCheckbox } from "./utils/elements.js";
import { connect } from "./utils/ws.js";
import { toTitle, formatNumber, frenchLocale, round } from "./utils/misc.js";

/*********/
/* model */
/*********/

export function initModel() {
  return {
    loading: false,
    open: true,
    ws: null,
    colours: [
      "blue",
      "green",
      "red",
      "purple",
      "orange",
      "pink",
      "turquoise",
      "yellow",
    ],
    running: false,
    station: null,
    snowModels: null,
    availableModels: null,
    objectives: null,
    transformations: null,
    algorithms: null,
    snowModel:
      window.localStorage.getItem("snow_model") == ""
        ? null
        : window.localStorage.getItem("snow_model") || null,
    models:
      window.localStorage.getItem("models") === null
        ? []
        : window.localStorage.getItem("models").split(","),
    objective: window.localStorage.getItem("objective") || null,
    transformation: window.localStorage.getItem("transformation") || null,
    algorithm: window.localStorage.getItem("algorithm") || null,
    algorithmParams: null,
    observations: null,
    predictions: null,
  };
}

export const initialMsg = [
  {
    type: "CalibrationMsg",
    data: { type: "Connect" },
  },
];

/**********/
/* update */
/**********/

export async function update(
  model,
  msg,
  dispatch,
  createNotification,
  station,
  petModel,
  nValidYears,
) {
  dispatch = createDispatch(dispatch);
  const configValid =
    station !== null && petModel !== null && nValidYears !== null;
  switch (msg.type) {
    case "CheckEscape":
      return model;
    case "SelectSection":
      const open = msg.data === "calibration";
      return { ...model, open: open };
    case "UpdateStation":
      return model;
    case "Connect":
      connect("calibration/", handleMessage, dispatch, createNotification);
      return { ...model, loading: true };
    case "Connected":
      if (model.snowModels === null || model.availableModels) {
        dispatch({ type: "GetModels" });
      }
      if (model.observations === null) {
        dispatch({ type: "GetObservations" });
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
      if (model.objective === null) {
        dispatch({ type: "UpdateObjective", data: msg.data.objectives[0] });
      }
      if (model.transformation === null) {
        dispatch({
          type: "UpdateTransformation",
          data: msg.data.transformations[0],
        });
      }
      if (model.algorithm === null) {
        dispatch({
          type: "UpdateAlgorithm",
          data: Object.keys(msg.data.algorithms)[0],
        });
      } else {
        dispatch({ type: "UpdateAlgorithm", data: model.algorithm });
      }
      if (model.models.length === 0) {
        dispatch({
          type: "UpdateModels",
          data: { model: msg.data.climate[0], checked: true },
        });
      }
      return {
        ...model,
        loading: false,
        snowModels: msg.data.snow,
        availableModels: msg.data.climate,
        objectives: msg.data.objectives,
        transformations: msg.data.transformations,
        algorithms: msg.data.algorithms,
      };
    case "UpdateModels":
      const models = msg.data.checked
        ? [...model.models, msg.data.model]
        : model.models.filter((m) => m !== msg.data.model);
      if (models.length === 0) {
        window.localStorage.setItem("models", "");
      } else {
        window.localStorage.setItem("models", models.join(","));
      }
      return { ...model, models: models };
    case "UpdateSnowModel":
      window.localStorage.setItem("snow_model", msg.data);
      return { ...model, snowModel: msg.data == "" ? null : msg.data };
    case "UpdateObjective":
      window.localStorage.setItem("objective", msg.data);
      return { ...model, objective: msg.data };
    case "UpdateTransformation":
      window.localStorage.setItem("transformation", msg.data);
      return { ...model, transformation: msg.data };
    case "UpdateAlgorithm":
      window.localStorage.setItem("algorithm", msg.data);
      const config = model.algorithms[model.algorithm];
      const params =
        config === undefined
          ? {}
          : Object.fromEntries(
              Object.entries(config).map(([param, values]) => [
                param,
                window.localStorage.getItem(`${msg.data}__${param}`) === null
                  ? values.default
                  : values.step === 1
                    ? parseInt(
                        window.localStorage.getItem(`${msg.data}__${param}`),
                      )
                    : parseFloat(
                        window.localStorage.getItem(`${msg.data}__${param}`),
                      ),
              ]),
            );
      return { ...model, algorithm: msg.data, algorithmParams: params };
    case "UpdateParam":
      if (model.algorithm === null) {
        return model;
      } else {
        window.localStorage.setItem(
          `${model.algorithm}__${msg.data.param}`,
          msg.data.value,
        );
        return {
          ...model,
          algorithmParams: {
            ...model.algorithmParams,
            [msg.data.param]: msg.data.value,
          },
        };
      }
    case "StartCalibration":
      if (model.ws?.readyState === WebSocket.OPEN && configValid) {
        model.models.forEach((m) => {
          model.ws.send(
            JSON.stringify({
              type: "calibration_start",
              data: {
                station: station,
                pet_model: petModel,
                n_valid_years: nValidYears,
                climate_model: m,
                snow_model: model.snowModel,
                objective: model.objective,
                transformation: model.transformation,
                algorithm: model.algorithm,
                algorithm_params: model.algorithmParams,
              },
            }),
          );
        });
      } else {
        setTimeout(() => dispatch(msg), 1000);
      }
      return { ...model, loading: true, running: true };
    case "StopCalibration":
      if (model.ws?.readyState === WebSocket.OPEN) {
        if (msg.data) {
          model.ws.send(
            JSON.stringify({
              type: "calibration_stop",
              data: msg.data,
            }),
          );
        } else {
          model.models.forEach((m) => {
            model.ws.send(
              JSON.stringify({
                type: "calibration_stop",
                data: m,
              }),
            );
          });
        }
      }

      return { ...model, loading: false, running: false };
    case "GotCalibrationStep":
      if (model.running) {
        const modelPredictions =
          model.predictions === null
            ? []
            : (model.predictions[msg.data.model] ?? []);
        modelPredictions.push({
          predictions: msg.data.predictions,
          results: msg.data.results,
        });
        if (msg.data.done) {
          dispatch({ type: "StopCalibration", data: msg.data.model });
        }
        return {
          ...model,
          predictions: {
            ...(model.predictions ?? {}),
            [msg.data.model]: modelPredictions,
          },
        };
      } else {
        return model;
      }
    case "GetObservations":
      if (model.ws?.readyState === WebSocket.OPEN && configValid) {
        model.ws.send(
          JSON.stringify({
            type: "observations",
            data: {
              station: station,
              pet_model: petModel,
              n_valid_years: nValidYears,
            },
          }),
        );
      } else {
        setTimeout(() => dispatch(msg), 1000);
      }
      return { ...model, loading: true };
    case "GotObservations":
      return {
        ...model,
        loading: false,
        observations: msg.data.observations,
        predictions: {
          ...(model.predictions ?? {}),
          day_median: msg.data.day_median,
        },
      };
    default:
      return model;
  }
}

function createDispatch(dispatch) {
  return (msg) => dispatch({ type: "CalibrationMsg", data: msg });
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
    case "observations":
      dispatch({ type: "GotObservations", data: msg.data });
      break;
    case "calibration_step":
      dispatch({ type: "GotCalibrationStep", data: msg.data });
      break;
    default:
      createNotification("Unknown websocket message", true);
      break;
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
  formView(model, dispatch);
  calibrationView(model, dispatch);
}

function openView(model) {
  if (model.open) {
    document.getElementById("calibration-meta").classList.add("open");
    document.getElementById("calibration-main").classList.add("open");
  } else {
    document.getElementById("calibration-meta").classList.remove("open");
    document.getElementById("calibration-main").classList.remove("open");
  }
}

function initMetaView(model, globalDispatch) {
  document.getElementById("meta").appendChild(
    create(
      "div",
      { id: "calibration-meta", class: model.open ? "open" : "" },
      [
        create("h2", {}, ["Calibration"]),
        create("span", {}, "Modèles climatiques:"),
        create("div", { id: "calibration__models" }, []),
        create("span", { class: "calibration__snow" }, "Modèle de neige:"),
        create(
          "span",
          { id: "calibration__snow", class: "calibration__snow" },
          [],
        ),
      ],
      [
        {
          event: "click",
          fct: () => {
            globalDispatch({ type: "SelectSection", data: "calibration" });
          },
        },
      ],
    ),
  );
}

function initMainView(model, dispatch) {
  document.getElementById("main").appendChild(
    create(
      "section",
      { id: "calibration-main", class: model.open ? "open" : "" },
      [
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
            create("div", {}, [
              create("label", {}, ["Modèles climatiques:"]),
              create("div", { id: "calibration-selection__models" }, []),
              create("label", { for: "calibration-selection__snow" }, [
                "Neige:",
              ]),
              create(
                "select",
                { id: "calibration-selection__snow", hidden: true },
                [],
                [
                  {
                    event: "input",
                    fct: (event) => {
                      dispatch({
                        type: "UpdateSnowModel",
                        data: event.target.value,
                      });
                    },
                  },
                ],
              ),
              create("label", { for: "calibration-selection__objective" }, [
                "Fonction d'objectif:",
              ]),
              create(
                "select",
                { id: "calibration-selection__objective", hidden: true },
                [],
                [
                  {
                    event: "input",
                    fct: (event) => {
                      dispatch({
                        type: "UpdateObjective",
                        data: event.target.value,
                      });
                    },
                  },
                ],
              ),
              create(
                "label",
                { for: "calibration-selection__transformation" },
                ["Fonction de transformation de débit:"],
              ),
              create(
                "select",
                { id: "calibration-selection__transformation", hidden: true },
                [],
                [
                  {
                    event: "input",
                    fct: (event) => {
                      dispatch({
                        type: "UpdateTransformation",
                        data: event.target.value,
                      });
                    },
                  },
                ],
              ),
            ]),
            create("div", {}, [
              create("label", { for: "calibration-selection__algorithm" }, [
                "Algorithme d'optimisation:",
              ]),
              create(
                "select",
                { id: "calibration-selection__algorithm", hidden: true },
                [],
                [
                  {
                    event: "input",
                    fct: (event) => {
                      dispatch({
                        type: "UpdateAlgorithm",
                        data: event.target.value,
                      });
                    },
                  },
                ],
              ),
              create(
                "div",
                { id: "calibration-selection__algorithm-params" },
                [],
              ),
            ]),
            create("div", { class: "break" }),
            create(
              "button",
              { id: "calibration_start-button" },
              ["Partir calibration"],
              [
                {
                  event: "click",
                  fct: () => {
                    dispatch({ type: "StartCalibration" });
                  },
                },
              ],
            ),
            create(
              "button",
              { id: "calibration_stop-button", hidden: true },
              ["Arrêter calibration"],
              [
                {
                  event: "click",
                  fct: () => {
                    dispatch({ type: "StopCalibration" });
                  },
                },
              ],
            ),
          ],
          [{ event: "submit", fct: (event) => event.preventDefault() }],
        ),
        create("div", { id: "calibration-main__plots" }, [
          create("div", { id: "calibration-main__legend" }, []),
          create("svg", { id: "calibration-main__discharge", class: "plot" }),
          create("svg", { id: "calibration-main__rmse", class: "plot" }),
          create("svg", { id: "calibration-main__nse", class: "plot" }),
          create("svg", { id: "calibration-main__kge", class: "plot" }),
        ]),
      ],
    ),
  );

  let resizeTimeout;
  const resizeObserver = new ResizeObserver(() => {
    clearTimeout(resizeTimeout);
    resizeTimeout = setTimeout(() => dispatch({ type: "Noop" }), 100);
  });
  resizeObserver.observe(document.getElementById("data-main__plots"));
}

function metaView(model) {
  if (model.snowModel === null) {
    document.getElementById("calibration__snow").textContent = "Aucun";
    [...document.querySelectorAll(".calibration__snow")].forEach((span) =>
      span.classList.add("disabled"),
    );
  } else {
    document.getElementById("calibration__snow").textContent = toTitle(
      model.snowModel,
    );
    [...document.querySelectorAll(".calibration__snow")].forEach((span) =>
      span.classList.remove("disabled"),
    );
  }

  const models = document.getElementById("calibration__models");
  clear(models);
  model.models.forEach((m) => {
    models.appendChild(create("span", {}, m));
  });
}

function formView(model, dispatch) {
  const modelsDiv = document.getElementById("calibration-selection__models");
  if (modelsDiv.children.length === 0 && model.availableModels !== null) {
    model.availableModels.forEach((_model) => {
      modelsDiv.appendChild(
        create("label", { for: `calibration-selection__models__${_model}` }, [
          _model,
        ]),
      );
      modelsDiv.appendChild(
        createCheckbox(
          {
            id: `calibration-selection__models__${_model}`,
            ...(model.models.includes(_model) ? { checked: true } : {}),
          },
          [
            {
              event: "change",
              fct: (event) =>
                dispatch({
                  type: "UpdateModels",
                  data: { model: _model, checked: event.target.checked },
                }),
            },
          ],
        ),
      );
    });
  }

  addSelectOptions(
    model.snowModels,
    model.snowModel,
    "calibration-selection__snow",
    (n) => (n === null ? "Aucun" : toTitle(n)),
  );
  addSelectOptions(
    model.objectives,
    model.objective,
    "calibration-selection__objective",
    (n) => n,
  );
  addSelectOptions(
    model.transformations,
    model.transformation,
    "calibration-selection__transformation",
    (n) =>
      ({
        log: "Low flows: log",
        sqrt: "Medium flows: sqrt",
        none: "High flows: none",
      })[n] ?? n,
  );
  addSelectOptions(
    model.algorithms === null ? null : Object.keys(model.algorithms),
    model.algorithm,
    "calibration-selection__algorithm",
    (n) => n,
  );

  addAlgorithmOptions(model, dispatch);
}

function addSelectOptions(values, current, id, toName) {
  const select = document.getElementById(id);
  if (select.children.length === 0 && values !== null) {
    select.removeAttribute("hidden");
    values.forEach((val) => {
      const option = create("option", { value: val === null ? "" : val }, [
        toName(val),
      ]);
      option.selected = val === current;
      select.appendChild(option);
    });
  }
}

function addAlgorithmOptions(model, dispatch) {
  const div = document.getElementById(
    "calibration-selection__algorithm-params",
  );

  if (
    model.algorithms !== null &&
    model.algorithm !== null &&
    model.algorithmParams !== null
  ) {
    const params = model.algorithms[model.algorithm];
    if (params === undefined) {
      clear(div);
    } else {
      if (div.children.length !== 2 * Object.keys(params).length) {
        clear(div);
        Object.entries(params).forEach(([param, values]) => {
          if (
            document.getElementById(
              `calibration-selection__algorithm-params__${param}`,
            ) === null
          ) {
            div.appendChild(
              create(
                "label",
                { for: `calibration-selection__algorithm-params__${param}` },
                [`${param}:`],
              ),
            );
            div.appendChild(
              create(
                "input",
                {
                  id: `calibration-selection__algorithm-params__${param}`,
                  type: "number",
                  step: values.step,
                  value:
                    model.algorithmParams[param] === undefined
                      ? values.default
                      : model.algorithmParams[param],
                  ...(values.min === null ? {} : { min: values.min }),
                  ...(values.max === null ? {} : { max: values.max }),
                },
                [],
                [
                  {
                    event: "input",
                    fct: (event) => {
                      dispatch({
                        type: "UpdateParam",
                        data: { param: param, value: event.target.value },
                      });
                    },
                  },
                ],
              ),
            );
          }
        });
      }
    }
  } else {
    clear(div);
  }
}

function calibrationView(model, dispatch) {
  if (model.running) {
    document
      .getElementById("calibration_start-button")
      .setAttribute("hidden", true);
    document
      .getElementById("calibration_stop-button")
      .removeAttribute("hidden");
  } else {
    document
      .getElementById("calibration_start-button")
      .removeAttribute("hidden");
    document
      .getElementById("calibration_stop-button")
      .setAttribute("hidden", true);
  }

  legendView(model, dispatch);
  dischargeView(model);
  metricView(model, "rmse");
  metricView(model, "nse");
  metricView(model, "kge");
}

function legendView(model, dispatch) {
  const legend = document.getElementById("calibration-main__legend");
  clear(legend);

  if (model.observations !== null) {
    legend.appendChild(
      create("div", {}, [
        create("span", {}, ["observations"]),
        create("span", { class: model.colours[0] }),
      ]),
    );
  }

  if (model.predictions !== null) {
    Object.entries(model.predictions).forEach(([m, _], i) => {
      legend.appendChild(
        create("div", {}, [
          create("span", {}, [m.replace("_", " ")]),
          create("span", { class: model.colours[i + 1] }),
        ]),
      );
    });
  }
}

function dischargeView(model) {
  const _svg = document.getElementById("calibration-main__discharge");
  clear(_svg);
  if (model.observations === null) {
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
      b: height - 50,
    };

    const svg = d3.select(_svg);

    const observations = model.observations;

    const xScale = d3
      .scaleTime()
      .domain(d3.extent(observations, (d) => new Date(d.date)))
      .range([boundaries.l, boundaries.r]);
    const yScale = d3
      .scaleLinear()
      .domain([
        d3.min(observations, (d) => d.discharge),
        d3.max(observations, (d) => d.discharge),
      ])
      .range([boundaries.b, boundaries.t]);

    // x axis
    const xAxis = svg
      .append("g")
      .attr("class", "x-axis")
      .attr("transform", `translate(0, ${boundaries.b})`)
      .call(d3.axisBottom(xScale).tickFormat(frenchLocale.format("%Y")));
    xAxis
      .selectAll("text")
      .attr("transform", "rotate(-45)")
      .attr("text-anchor", "end")
      .attr("dx", "-0.5em")
      .attr("dy", "0.5em");
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
      .text("Débit");

    // observations
    svg
      .append("path")
      .attr("class", model.colours[0])
      .datum(observations)
      .attr(
        "d",
        d3
          .line()
          .x((d) => xScale(new Date(d.date)))
          .y((d) => yScale(d.discharge)),
      );

    // predictions
    if (model.predictions !== null) {
      Object.entries(model.predictions).forEach(([m, { predictions }], i) => {
        svg
          .append("path")
          .attr("class", model.colours[i + 1])
          .datum(predictions)
          .attr(
            "d",
            d3
              .line()
              .x((d) => xScale(new Date(d.date)))
              .y((d) => yScale(d.discharge)),
          );
      });
    }
  }
}

function metricView(model, metric) {
  const _svg = document.getElementById(`calibration-main__${metric}`);
  clear(_svg);
  if (model.predictions === null) {
    _svg.setAttribute("hidden", true);
  } else {
    _svg.removeAttribute("hidden");
    const width = _svg.clientWidth;
    const height = _svg.clientHeight;
    _svg.setAttribute("viewBox", `0 0 ${width} ${height}`);

    const boundaries = {
      l: 25,
      r: width - 5,
      t: 15,
      b: height - 50,
    };

    const svg = d3.select(_svg);

    const data = Object.entries(model.predictions).map(
      ([m, { results }], i) => ({
        model: m,
        value: results[metric],
        colour: model.colours[i + 1],
      }),
    );

    const xScale = d3
      .scaleBand()
      .domain(d3.extent(data, (d) => d.model))
      .range([boundaries.l, boundaries.r])
      .padding(0.1);
    const yScale = d3
      .scaleLinear()
      .domain([
        Math.min(
          0,
          d3.min(data, (d) => d.value),
        ),
        Math.max(
          1,
          d3.max(data, (d) => d.value),
        ),
      ])
      .range([boundaries.b, boundaries.t]);

    // x axis
    const xAxis = svg
      .append("g")
      .attr("class", "x-axis")
      .attr("transform", `translate(0, ${boundaries.b})`)
      .call(d3.axisBottom(xScale));
    xAxis.selectAll("text").remove();
    // y axis
    const yAxis = svg
      .append("g")
      .attr("class", "y-axis")
      .attr("transform", `translate(${boundaries.l}, 0)`)
      .call(d3.axisLeft(yScale).ticks(0));
    yAxis.selectAll("text").remove();
    svg
      .append("text")
      .attr("x", 10)
      .attr("y", (boundaries.t + boundaries.b) / 2)
      .attr("text-anchor", "middle")
      .attr("dominant-baseline", "middle")
      .attr(
        "transform",
        `rotate(-90, 10, ${(boundaries.t + boundaries.b) / 2})`,
      )
      .attr("font-size", "0.9rem")
      .text(metric);

    // bands
    svg
      .selectAll("rect")
      .data(data)
      .join("rect")
      .attr("class", (d) => d.colour)
      .attr("x", (d) => xScale(d.model))
      .attr("y", (d) => yScale(d.value))
      .attr("width", xScale.bandwidth())
      .attr("height", (d) => yScale(0) - yScale(d.value));
    // values
    svg
      .append("g")
      .selectAll("text")
      .attr("class", "text-values")
      .data(data)
      .join("text")
      .attr("x", (d) => xScale(d.model) + xScale.bandwidth() / 2)
      .attr("y", (d) => yScale(d.value))
      .attr("dy", -5)
      .attr("text-anchor", "middle")
      .attr("font-size", "0.9rem")
      .text((d) => round(d.value, 2));
  }
}
