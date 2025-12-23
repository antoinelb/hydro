import { create } from "./utils.js";

import * as settings from "./settings.js";
import * as notifications from "./notifications.js";

/*********/
/* model */
/*********/

function initModel() {
  return {
    preventEscape: false,
    loading: false,
    settings: settings.initModel(),
    notifications: notifications.initModel(),
  };
}

const initialMsg = null;

/**********/
/* update */
/**********/

async function update(model, msg, dispatch) {
  switch (msg.type) {
    case "CheckEscape":
      dispathCheckEscape(model, msg.data, dispatch);
      return model;
    case "SettingsMsg":
      return {
        ...model,
        settings: await settings.update(model.settings, msg.data, dispatch),
      };
    case "NotificationsMsg":
      return {
        ...model,
        notifications: await notifications.update(
          model.notifications,
          msg.data,
          dispatch,
        ),
      };
    default:
      return model;
  }
}

function dispathCheckEscape(model, event, dispatch) {
  if (!model.preventEscape) {
    if (
      event.type === "click" ||
      (event.type === "keydown" && event.key === "Escape")
    ) {
      ["SettingsMsg", "NotificationsMsg"].forEach((msg) => {
        dispatch({ type: msg, data: { type: "CheckEscape", data: event } });
      });
    }
  }
}

/********/
/* view */
/********/

async function initView(dispatch) {
  await injectSvgSprite();
  document.body.append(
    settings.initView(dispatch),
    notifications.initView(dispatch),
    create("main"),
  );
  document.body.addEventListener("click", (event) =>
    dispatch({ type: "CheckEscape", data: event }),
  );
  document.body.addEventListener("keydown", (event) =>
    dispatch({ type: "CheckEscape", data: event }),
  );
}

async function injectSvgSprite() {
  if (!document.getElementById("svg-sprite")) {
    const resp = await fetch("/static/assets/icons/icons.svg");
    const sprite = await resp.text();
    document.body.insertAdjacentHTML("beforebegin", sprite);
  }
}

function view(msg, model, dispatch) {
  switch (msg.type) {
    case "SettingsMsg":
      settings.view(model.settings, dispatch);
      break;
    case "notificationsMsg":
      notifications.view(model.notifications, dispatch);
      break;
  }
  loadingView(model);
}

function loadingView(model) {
  const loading =
    model.loading || model.settings.loading || model.notifications.loading;
  if (loading) {
    if (
      document.querySelector("link[rel~='icon']").href !==
      "/static/assets/icons/loading.svg"
    ) {
      document.querySelector("link[rel~='icon']").href =
        "/static/assets/icons/loading.svg";
    }
  } else {
    if (
      document.querySelector("link[rel~='icon']").href !==
      "/static/assets/icons/favicon.svg"
    ) {
      document.querySelector("link[rel~='icon']").href =
        "/static/assets/icons/favicon.svg";
    }
  }
}

/********/
/* init */
/********/

async function init() {
  let queue = [];
  let processing = false;

  let model = initModel();

  const processQueue = async () => {
    processing = true;
    while (queue.length > 0) {
      const msg = queue.shift();
      model = await update(model, msg, dispatch);
      // console.log(msg, model);
      view(msg, model, dispatch);
    }
    processing = false;
  };

  const dispatch = async (msg) => {
    queue.push(msg);
    if (!processing) {
      processQueue();
    }
  };

  await initView(dispatch);
  [initialMsg, settings.initialMsg, notifications.initialMsg].forEach((msg) => {
    if (msg) {
      dispatch(msg);
    }
  });
}

window.addEventListener("load", init);
