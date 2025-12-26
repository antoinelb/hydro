import { create } from "./utils/elements.js";

/*********/
/* model */
/*********/

export function initModel() {
  return {
    notifications: [],
  };
}

export const initialMsg = null;

/**********/
/* update */
/**********/

export async function update(model, msg, dispatch) {
  dispatch = createDispatch(dispatch);
  switch (msg.type) {
    case "AddNotification":
      const notification = {
        id:
          model.notifications.length === 0
            ? 0
            : Math.max(...model.notifications.map((n) => n.id)) + 1,
        text: msg.data.text,
        isError: msg.data.isError,
      };
      setTimeout(
        () => dispatch({ type: "RemoveNotification", data: notification.id }),
        3000,
      );
      return {
        ...model,
        notifications: [...model.notifications, notification],
      };
    case "RemoveNotification":
      return {
        ...model,
        notifications: model.notifications.filter(
          (notification) => notification.id !== msg.data,
        ),
      };
    default:
      return model;
  }
}

function createDispatch(dispatch) {
  return (msg) => dispatch({ type: "NotificationsMsg", data: msg });
}

/********/
/* view */
/********/

export function initView(_) {
  return create("div", { id: "notifications" });
}

export function view(model, dispatch) {
  dispatch = createDispatch(dispatch);

  const div = document.getElementById("notifications");
  const notifications = [...document.querySelectorAll(".notification")];
  const current = new Set(model.notifications.map((n) => n.id));
  const toRemove = new Set(
    notifications.map((n) => n.getAttribute("data-id")),
  ).difference(current);
  const toAdd = current.difference(toRemove);

  notifications.forEach((n) => {
    if (toRemove.has(n.getAttribute("data-id"))) {
      div.removeChild(n);
    }
  });
  model.notifications.forEach((n) => {
    if (toAdd.has(n.id)) {
      div.appendChild(
        create(
          "div",
          { class: `notification ${n.isError ? "error" : ""}` },
          [create("span", {}, n.text), create("div", {}, create("div"))],
          [
            {
              event: "click",
              fct: () => dispatch({ type: "RemoveNotification", data: n.id }),
            },
          ],
        ),
      );
    }
  });
}
