import themes from "./themes.js";
import changeTheme from "./themeing.js";
import run from "./run.js";

requireAuth(() => loadCode());

const createSelector = (id, data, callback, defaultValue) => {
    let selector = $(`select#${id}`);
    selector.html(Object.keys(data).map(item => `<option value="${item}">${item}</option >`));
    selector.change(selector => callback(selector.target.value));

    selector.val(defaultValue);
    callback(defaultValue);
};

// Settings
$("#settings-button").click(function (event) {
    event.preventDefault();
    this.blur();
    $("#settings-modal").modal({
        showClose: false
    });
});

let langTools = ace.require("ace/ext/language_tools");
var editor = ace.edit("editor");
editor.setOptions({
    copyWithEmptySelection: true,
    enableBasicAutocompletion: true,
    enableLiveAutocompletion: true,
    enableSnippets: true,
});

// var snippetManager = ace.require("ace/snippets").snippetManager;
// snippetManager.insertSnippet(editor, snippet);

// var rhymeCompleter = {
//     getCompletions: function(editor, session, pos, prefix, callback) {
//         if (prefix.length === 0) { callback(null, []); return }
//         $.getJSON(
//             "http://rhymebrain.com/talk?function=getRhymes&word=" + prefix,
//             function(wordList) {
//                 // wordList like [{"word":"flow","freq":24,"score":300,"flags":"bc","syllables":"1"}]
//                 callback(null, wordList.map(function(ea) {
//                     return {name: ea.word, value: ea.word, score: ea.score, meta: "rhyme"}
//                 }));
//             })
//     }
// }
// langTools.addCompleter(rhymeCompleter);

// Selectors
const languages = {
    // "Lua": "ace/mode/lua",
    // "BASH": "ace/mode/bash",
    "JavaScript": "ace/mode/javascript",
    // "Python": "ace/mode/python",
    // "Rust": "ace/mode/rust",
    // "C": "ace/mode/c_cpp",
    // "C++": "ace/mode/c_cpp",
};

const keybindSchemes = {
    "Ace": null,
    "Vim": "ace/keyboard/vim",
    "Emacs": "ace/keyboard/emacs",
    "Sublime": "ace/keyboard/sublime",
    "VSCode": "ace/keyboard/vscode",
};

const changeKeybinds = (scheme) => {
    editor.setOption("keyboardHandler", keybindSchemes[scheme]);
};

const changeLanguage = (language) => {
    editor.session.setMode(languages[language]);
};

createSelector("theme-selector", themes, theme => changeTheme(editor, theme), "Monokai");
createSelector("keybinds-selector", keybindSchemes, changeKeybinds, "VSCode");
createSelector("language-selector", languages, changeLanguage, "JavaScript");

// Font size
const setFontSize = (size) => {
    editor.setOption("fontSize", size);
};

let fontSizeInput = $(`input#font-size-input`);
fontSizeInput.change(fontSizeInput => setFontSize(parseInt(fontSizeInput.target.value)));
fontSizeInput.val(14);
setFontSize(14);

// Console
$("input#console-input").submit(line => {
    console.log(line);
});

// Run
$("button#run-button").click(() => {
    run(editor.getValue(), editor.session.getMode().$id);
});

// Save
var db = firebase.firestore();

$("button#save-button").click(saveCode);
$(document).keydown(function (event) {
    if (event.ctrlKey && event.key === 's') {
        event.preventDefault();
        saveCode();
    }
});

function saveCode() {
    db.collection("users").doc(`${account.uid}/code/${editor.session.getMode().$id.split('/').pop()}`).set({
        text: editor.getValue(),
    }).catch(function (error) {
        window.error("Error saving code: ", error);
    });
}

function loadCode() {
    db.collection("users").doc(`${account.uid}/code/${editor.session.getMode().$id.split('/').pop()}`).get()
        .then(function (doc) {
            if (doc.exists) {
                editor.setValue(doc.data().text);
            } else {
                const templates = {
                    "ace/mode/javascript": "javascript.js",
                }
                fetch(`controller/template/${templates[editor.session.getMode().$id]}`)
                    .then(response => response.text())
                    .then(text => editor.setValue(text))
                    .catch(() => window.error("Error loading template code"));
            }
        }).catch(function (error) {
            window.error("Error loading code: ", error);
        });
}
