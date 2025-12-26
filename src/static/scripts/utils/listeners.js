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
