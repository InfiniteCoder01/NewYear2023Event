const firebaseApp = firebase.initializeApp({
    apiKey: "AIzaSyDhJqPdgfyPnRfIAyQXf7WlfRoYnjyMHOc",
    authDomain: "infinitecoderwebsite.firebaseapp.com",
    projectId: "infinitecoderwebsite",
    storageBucket: "infinitecoderwebsite.appspot.com",
    messagingSenderId: "445308018066",
    appId: "1:445308018066:web:692f0e3099df2ec6e7b54b",
    measurementId: "G-DSCZDK8FHL"
});

let account;
const requireAuth = (callback) => {
    firebase.auth().onAuthStateChanged(user => {
        if (user) {
            account = user.multiFactor.user;
            if (callback) callback();
        } else {
            document.location.href = `${document.location.origin}/account/sign-in.html?destination=${document.location.href}`;
        }
    });
};