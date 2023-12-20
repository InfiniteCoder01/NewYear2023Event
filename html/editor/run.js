// --------------------------------------- APIs --------------------------------------- //
function print(text) {
    $('#console').append(`<div>${text}</div>`)
};

function error(text) {
    $('#console').append(`<pre style=\"color: red;\">${text}</pre>`)
};

// let socket;
// window.registerClient = (callback) => {
//     if (socket != null) {
//         socket.close();
//     }
//     socket = new WebSocket(
//         `${window.location.protocol === 'https:' ? 'wss' : 'ws'}://${document.location.host}/connect/tetro/${account.uid}`
//     );

//     socket.onmessage = msg => {
//         if (typeof msg.data === "string") {
//             if (msg.data.startsWith("!")) {
//                 error(msg.data.substring(1));
//             } else {
//                 print(msg.data);
//             }
//             return;
//         }
//         console.log(callback);
//         processMessage(msg, callback);
//     };
// }

// window.sendMessage = (message) => {
//     try {
//         socket.send(message);
//     } catch (_) {
//         return false;
//     }
//     return true;
// }

// window.vec2 = function (x, y) { return { x, y }; };

// console.error = (...data) => error(data[0]);

// --------------------------------------- Run --------------------------------------- //
setInterval(() => $('#console').scrollTop($('#console')[0].scrollHeight), 25);

let worker;
const run = (code, language) => {
    // for (let task of backgroundTasks) {
    //     clearInterval(task);
    // }
    // backgroundTasks = [];
    $('#console').empty();

    // if (socket != null) {
    //     socket.close();
    //     socket = null;
    // }

    if (worker) worker.terminate();
    worker = new Worker("editor/worker.js");
    worker.onmessage = message => {
        if (message.data.error != null) error(message.data.error);
        else print(message.data);
    };
    worker.onerror = err => error(err.message);

    worker.postMessage({
        code,
        language,
        connectionURL: `${window.location.protocol === 'https:' ? 'wss' : 'ws'}://${document.location.host}/connect/$NAME/${account.uid}`
    });
};

export default run;