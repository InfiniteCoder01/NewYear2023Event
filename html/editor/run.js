import init, { eval_python } from "./pkg/web_editor.js";

init().then(() => {
    // greet("WebAssembly");
    eval_python("print(\"PyPy!\");");
});

const run = (code, language) => {
    if (language == "ace/mode/javascript") {
        const prelude = String.raw`
        `;

        // // Updated the position on the DOM 
        // const updatePosition = (element, randomPosition) => { 
        //     element.style.top = randomPosition[0] + "px"; 
        //     element.style.left = randomPosition[1] + "px"; 
        // } 
  
        // // Calculates the random position 
        // const getRandomPosition = 
        //     (height, width) => 
        //         [(Math.floor(Math.random() * height)), 
        //         Math.floor(Math.random() * width)]; 
  
        // // Creating a Web Worker 
        const worker = new Worker('editor/worker.js'); 
  
        // // Getting the GeeksForGeeks text 
        // const title = document.querySelector('h1'); 
  
        // // Updated the position on receiving  
        // // the random position 
        // worker.onmessage = (event) => 
        //     updatePosition(title, event.data); 
  
        // Passing the function to the Web Worker 
        worker.postMessage({ 
            function: code,
  
            // Arguments passed to the function 
            // arguments: [document.body.offsetHeight, 
            // document.body.offsetWidth, 1000] 
        }) 
        // eval(prelude + code);
    }
};

export default run;
// 'Hello from \x1B[1;3;31mxterm.js\x1B[0m $ '