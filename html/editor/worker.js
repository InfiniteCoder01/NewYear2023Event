self.onmessage = ({ data }) => {

    // Converting the string back to a string 
    const func = new Function(
        data.function
    )();

    // Evaluates the function and sends 
    // the data back to the main file 
    // const timerFunction = () => {
    //     randomPositions = func(
    //         data.arguments[0], data.arguments[1]
    //     );
    //     self.postMessage(randomPositions);
    // }

    // // Runs the timerFunction at every  
    // // interval specified in the arguments. 
    // setInterval(() => {
    //     timerFunction();
    // }, data.arguments[2])
// console.log(func());
}
