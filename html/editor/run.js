// --------------------------------------- APIs --------------------------------------- //
window.print = (text) => {
    $('#console').append(`<div>${text}</div>`)
};

window.error = (text) => {
    $('#console').append(`<pre style=\"color: red;\">${text}</pre>`)
};

let backgroundTasks = [];
window.repeat = function (callback, interval) {
    backgroundTasks.push(setInterval(callback, interval));
};

import processMessage from "/controller/api.js";

let socket;
window.registerClient = (callback) => {
    if (socket != null) {
        socket.close();
    }
    socket = new WebSocket(
        `${window.location.protocol === 'https:' ? 'wss' : 'ws'}://${document.location.host}/connect/tetro/${account.uid}`
    );

    socket.onmessage = msg => {
        if (typeof msg.data === "string") {
            if (msg.data.startsWith("!")) {
                error(msg.data.substring(1));
            } else {
                print(msg.data);
            }
            return;
        }
        processMessage(msg, callback);
    };
}

window.sendMessage = (message) => {
    try {
        socket.send(message);
    } catch (_) {
        return false;
    }
    return true;
}

window.vec2 = function (x, y) { return { x, y }; };

console.error = (...data) => error(data[0]);

// --------------------------------------- Run --------------------------------------- //
setInterval(() => $('#console').scrollTop($('#console')[0].scrollHeight), 25);

const run = (code, language) => {
    for (let task of backgroundTasks) {
        clearInterval(task);
    }
    backgroundTasks = [];
    $('#console').empty();

    if (socket != null) {
        socket.close();
        socket = null;
    }

    if (language == "ace/mode/javascript") {
        // eval(prelude + code);
    } else if (language == "ace/mode/python") {
        run_python(code);
    }

};

export default run;