import themes from "./themes.js";

const changeTheme = (editor, theme) => {
    const avgBrightness = (col) => {
        const regex = /([0-9]+),[\s]*([0-9]+),[\s]*([0-9]+)/;
        const [_, r, g, b] = regex.exec(col);
        return (parseInt(r) + parseInt(g) + parseInt(b)) / 3;
    };
    const lightenDarkenColor = (col, amt) => {
        const regex = /([0-9]+),[\s]*([0-9]+),[\s]*([0-9]+)/;
        const [_, r, g, b] = regex.exec(col);
        return `rgb(${parseInt(r) + amt}, ${parseInt(g) + amt}, ${parseInt(b) + amt})`;
    };

    editor.setTheme(`ace/theme/${themes[theme]}`, () => {
        let style = getComputedStyle($('div#editor')[0]);
        const isLight = avgBrightness(style.backgroundColor) > 128;
        style = {
            color: style.color,
            backgroundColor: style.backgroundColor,
            elementColor: (isLight ? lightenDarkenColor(style.backgroundColor, -10) : lightenDarkenColor(style.backgroundColor, 10)),
        };

        for (let container of $('.styled-container')) {
            container.style.backgroundColor = style.backgroundColor;
            container.style.color = style.color;
        }

        for (let item of $('.styled-item')) {
            if (item.tagName.toLowerCase() != "a" || item.classList.contains("abutton")) {
                item.style.backgroundColor = style.elementColor;
                item.style.color = style.color;
            }
        }
    });
};

export default changeTheme;