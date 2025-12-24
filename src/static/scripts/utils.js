export function create(type, attributes = {}, children = [], events = []) {
  const node =
    type === "svg" || type === "use"
      ? document.createElementNS("http://www.w3.org/2000/svg", type)
      : document.createElement(type);
  Object.keys(attributes).forEach((key) => {
    if (key === "style") {
      Object.keys(attributes[key]).forEach((style) => {
        node.style.setProperty(style, attributes[key][style]);
      });
    } else {
      node.setAttribute(key, attributes[key]);
    }
  });
  if (!Array.isArray(children)) {
    children = [children];
  }
  children.forEach((child) => {
    if (typeof child === "string" || typeof child === "number") {
      node.appendChild(document.createTextNode(child));
    } else {
      node.appendChild(child);
    }
  });
  events.forEach((event) => {
    node.addEventListener(event.event, event.fct);
  });
  return node;
}

export function onKey(key, callback, event, modifiers) {
  const withCtrl = modifiers ? modifiers.withCtrl || false : false;
  const withAlt = modifiers ? modifiers.withAlt || false : false;
  if (event.target.tagName !== "INPUT" && event.target.tagName !== "SELECT") {
    if (
      event.key === key &&
      event.ctrlKey === withCtrl &&
      event.altKey === withAlt
    ) {
      callback(event);
      event.preventDefault();
    }
  }
}

export function range(start, end) {
  if (end === undefined) {
    return [...Array(start).keys()];
  } else {
    return [...Array(end).keys()].filter((x) => x >= start);
  }
}

export function checkEscape(model, event, dispatch) {
  if (model.preventEscape) {
    return false;
  } else {
    if (event.type === "click") {
      return event.target.classList.contains("form__bg");
    } else if (event.type === "keydown") {
      if (event.key === "Escape") {
        const focused = document.activeElement;
        if (focused.tagName === "INPUT" || focused.tagName === "SELECT") {
          focused.blur();
          dispatch({ type: "SetPreventEscape" });
          return false;
        } else {
          return true;
        }
      } else {
        return false;
      }
    } else {
      return false;
    }
  }
}

export function clear(node) {
  [...node.children].forEach((child) => {
    node.removeChild(child);
  });
}

export function round(n, d) {
  return Math.round(n * 10 ** d) / 10 ** d;
}

export function formatNumber(n) {
  return n.toLocaleString("en-US").replace(/,/g, " ");
}

export function createSlider(
  id,
  min,
  max,
  isInteger,
  events = [],
  initialVal = null,
) {
  if (initialVal === null) {
    initialVal = isInteger
      ? Math.round((max + min) / 2)
      : round((max + min) / 2, 1);
  }

  return create("div", { class: "slider" }, [
    create(
      "input",
      {
        type: "range",
        min: min,
        max: max,
        step: isInteger ? "1" : "0.1",
        value: isInteger ? initialVal.toString() : initialVal.toFixed(1),
      },
      [],
      [
        ...events,
        {
          event: "input",
          fct: (event) => {
            document.getElementById(id).value = event.target.value;
          },
        },
      ],
    ),
    create(
      "input",
      {
        id: id,
        type: "number",
        min: min,
        max: max,
        step: isInteger ? "1" : "0.1",
        value: isInteger ? initialVal.toString() : initialVal.toFixed(1),
      },
      [],
      [
        {
          event: "input",
          fct: (event) => {
            setTimeout(() => {
              event.target.value = Math.min(
                Math.max(event.target.value, min),
                max,
              );
              const slider = event.target.parentNode.querySelector(
                "input[type='range']",
              );
              slider.value = event.target.value;
              slider.dispatchEvent(new Event("change", { bubbles: true }));
            }, 500);
          },
        },
      ],
    ),
  ]);
}

export function createLoading() {
  return create("svg", { class: "icon loading" }, [
    create("use", { href: "#icon-loader" }),
  ]);
}

export function createCheckbox(attributes, events) {
  return create("div", { class: "checkbox" }, [
    create("input", { type: "checkbox", ...attributes }, [], events),
    create("span"),
  ]);
}
