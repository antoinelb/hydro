import { create } from "./utils/elements.js";
import { onKey } from "./utils/listeners.js";

/*********/
/* model */
/*********/

export function initModel() {
  return {
    loading: false,
    open: false,
    theme: window.localStorage.getItem("settings--theme") || "dark",
    version: null,
  };
}

export const initialMsg = {
  type: "SettingsMsg",
  data: { type: "GetVersion" },
};

/**********/
/* update */
/**********/

export async function update(model, msg, dispatch) {
  dispatch = createDispatch(dispatch);
  switch (msg.type) {
    case "CheckEscape":
      if (
        msg.data.type === "click" &&
        document.getElementById("settings").contains(msg.data.target)
      ) {
        return model;
      } else {
        return { ...model, open: false };
      }
    case "GetVersion":
      getVersion(dispatch);
      return { ...model, loading: true };
    case "GotVersion":
      return { ...model, loading: false, version: msg.data };
    case "ToggleOpen":
      return { ...model, open: !model.open };
    case "ToggleTheme":
      const theme = model.theme === "dark" ? "light" : "dark";
      window.localStorage.setItem("settings--theme", theme);
      return { ...model, theme: theme };
    default:
      return model;
  }
}

function createDispatch(dispatch) {
  return (msg) => dispatch({ type: "SettingsMsg", data: msg });
}

async function getVersion(dispatch) {
  const resp = await fetch("/version");
  const version = await resp.text();
  dispatch({ type: "GotVersion", data: version });
}

/********/
/* view */
/********/

export function initView(dispatch) {
  dispatch = createDispatch(dispatch);
  document.addEventListener("keydown", (event) =>
    onKey(
      "T",
      async () =>
        await dispatch({
          type: "ToggleTheme",
        }),
      event,
    ),
  );
  return create("div", { id: "settings" }, [
    create(
      "button",
      { title: "Toggle settings" },
      [
        create("svg", { class: "icon" }, [
          create("use", { href: "#icon-menu" }),
        ]),
      ],
      [
        {
          event: "click",
          fct: () =>
            dispatch({
              type: "ToggleOpen",
            }),
        },
      ],
    ),
    create("div", {}, [
      create(
        "button",
        { id: "theme" },
        [
          create("svg", { id: "theme__moon", class: "icon" }, [
            create("use", { href: "#icon-moon" }),
          ]),
          create("svg", { id: "theme__sun", class: "icon" }, [
            create("use", { href: "#icon-sun" }),
          ]),
          create("span", {}, ["Changer thème"]),
          create("span", { class: "hotkey" }, ["T"]),
        ],
        [
          {
            event: "click",
            fct: async () =>
              await dispatch({
                type: "ToggleTheme",
              }),
          },
        ],
      ),
      create("div", { id: "version" }, [
        create("span", {}, ["Version: "]),
        create("span"),
      ]),
    ]),
  ]);
}

export function view(model, dispatch) {
  dispatch = createDispatch(dispatch);

  if (model.open) {
    document.getElementById("settings").classList.add("settings--open");
  } else {
    document.getElementById("settings").classList.remove("settings--open");
  }

  if (model.theme === "dark") {
    document.body.classList.remove("light");
  } else {
    document.body.classList.add("light");
  }

  if (
    document.querySelector("#version span:last-child").textContent !==
    model.version
  ) {
    document.querySelector("#version span:last-child").textContent =
      model.version;
  }
}
