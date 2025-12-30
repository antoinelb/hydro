import { create, clear, createCheckbox } from "./utils/elements.js";
import { connect } from "./utils/ws.js";
import { toTitle, formatNumber, frenchLocale } from "./utils/misc.js";

/*********/
/* model */
/*********/

export function initModel() {
  return {
    loading: false,
    open: true,
    ws: null,
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

export async function update(model, msg, dispatch, createNotification) {
  dispatch = createDispatch(dispatch);
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
        dispatch({ type: "UpdateAlgorithm", data: msg.data.algorithms[0] });
      } else {
        dispatch({ type: "UpdateAlgorithm", data: model.algorithm });
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
          ],
          [{ event: "submit", fct: (event) => event.preventDefault() }],
        ),
      ],
    ),
  );
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
