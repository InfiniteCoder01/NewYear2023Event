importScripts("/controller/api.js");

function print(message) {
    self.postMessage(message);
}

function error(message) {
    self.postMessage({ error: message });
}

let socket, websocketURL;

function connect(onMessage) {
    socket = new WebSocket(websocketURL.replace("$NAME", apiName));
    socket.onmessage = msg => {
        if (typeof msg.data === "string") {
            if (msg.data.startsWith("!")) {
                error(msg.data.substring(1));
            } else {
                print(msg.data);
            }
            return;
        }
        processMessage(msg, onMessage);
    };
}

self.onmessage = function (event) {
    const { code, language, connectionURL } = event.data;
    websocketURL = connectionURL;

    if (language == "ace/mode/javascript") {
        new Function(code)();
    } else if (language == "ace/mode/python") {
    }
};

onerror = function (event) {
    error(event);
}