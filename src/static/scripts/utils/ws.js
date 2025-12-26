export function connect(url, handleMessage, dispatch, globalDispatch) {
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const ws = new WebSocket(`${protocol}//${window.location.host}/${url}`);

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
