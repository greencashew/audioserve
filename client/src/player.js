import "./player.css";

export function formatTime(dur) {
    if (! isFinite(dur)) return "?";
    let hours = 0;
    let mins = Math.floor(dur / 60);
    let secs = Math.round(dur % 60);
    secs = ("0"+secs).slice(-2);
    if (mins >=60) {
        hours = Math.floor(mins / 60);
        mins = mins - hours * 60;
        mins = ("0"+mins).slice(-2);
    }
    if (hours) {
        return `${hours}:${mins}:${secs}`
    } else {
        return `${mins}:${secs}`;
    }
}

const VOLUME_FULL = 'M14.667 0v2.747c3.853 1.146 6.666 4.72 6.666 8.946 0 4.227-2.813 7.787-6.666 8.934v2.76C20 22.173 24 17.4 24 11.693 24 5.987 20 1.213 14.667 0zM18 11.693c0-2.36-1.333-4.386-3.333-5.373v10.707c2-.947 3.333-2.987 3.333-5.334zm-18-4v8h5.333L12 22.36V1.027L5.333 7.693H0z';
const VOLUME_MED = 'M0 7.667v8h5.333L12 22.333V1L5.333 7.667M17.333 11.373C17.333 9.013 16 6.987 14 6v10.707c2-.947 3.333-2.987 3.333-5.334z';
const VOLUME_LOW = 'M0 7.667v8h5.333L12 22.333V1L5.333 7.667';
const PLAY = "M18 12L0 24V0";
const PAUSE = "M0 0h6v24H0zM12 0h6v24h-6z";


export class AudioPlayer {
    // Most of code copied from https://codepen.io/gregh/pen/NdVvbm

    constructor() {

        this.unsized = false;
        this.knownDuration =null;

        let audioPlayer = document.querySelector('.audio-player');
        this._rootElem = audioPlayer;

        this._playPause = audioPlayer.querySelector('#playPause');
        this._playpauseBtn = audioPlayer.querySelector('.play-pause-btn');
        this._loading = audioPlayer.querySelector('.loading');
        this._progress = audioPlayer.querySelector('.progress');
        let volumeControls = audioPlayer.querySelector('.volume-controls');
        this._volumeProgress = volumeControls.querySelector('.slider .progress');
        this._player = audioPlayer.querySelector('audio');
        this._currentTime = audioPlayer.querySelector('.current-time');
        this._totalTime = audioPlayer.querySelector('.total-time');
        this._speaker = audioPlayer.querySelector('#speaker');
        this._currentlyDragged = null;

        let volumeBtn = audioPlayer.querySelector('.volume-btn');
        let sliderTime = audioPlayer.querySelector(".controls .slider");
        let sliderVolume = audioPlayer.querySelector(".volume .slider");
        let pinTime = sliderTime.querySelector(".pin");
        let pinVolume = sliderVolume.querySelector(".pin");


        pinTime.addEventListener('mousedown', (event) => {

            this._currentlyDragged = event.target;
            let handler = this._onMoveSlider.bind(this);
            window.addEventListener('mousemove', handler, false);
            window.addEventListener('mouseup', (evt) => {
                window.setTimeout(() => this._currentlyDragged = false, 200);
                this._onMoveSlider(evt, true);
                window.removeEventListener('mousemove', handler, false);
                evt.stopImmediatePropagation();
            }, { once: true });
        });

        let touchToEvent = (touch, type) => {
            return {
                target: touch.target,
                clientX: touch.clientX,
                clientY: touch.clientY,
                type: type
            }
        }


        window.addEventListener("touchcancel", () => {
            console.log("touch canceled");
        })

        pinTime.addEventListener("touchstart", (event) => {
            if (event.changedTouches.length == 1 && event.targetTouches.length ==1) {
                let touch = event.changedTouches[0];
                this._currentlyDragged = touch.target;
                let touchId = touch.identifier;
                let clientX, clientY;

                let myTouch = (event) => {
                    for (let i = 0; i< event.changedTouches.length; i++) {
                        let t = event.changedTouches.item(i);
                        if (t.identifier === touch.identifier) return t;
                    }
                }
                
                let handler = (event) => {
                    let t = myTouch(event);
                    if (t) {
                    let evt = touchToEvent(t, "mousemove");
                    clientX=evt.clientX;
                    clientY=evt.clientY;
                    this._onMoveSlider(evt);
                    }
                }
                window.addEventListener("touchmove", handler);
                window.addEventListener("touchend", (event) => {
                    let t = myTouch(event);
                    if (t) {
                    window.setTimeout( () => { this._currentlyDragged = false}, 200);
                    window.removeEventListener("touchmove", handler);
                    let evt = touchToEvent(event, "mouseup");
                    evt.clientX = clientX;
                    evt.clientY = clientY;
                    this._onMoveSlider(evt, true);
                    }

                }, {once:true});
            }
        }, {passive:true});

        pinVolume.addEventListener('mousedown', (event) => {

            this._currentlyDragged = event.target;
            let handler = this._onChangeVolume.bind(this);
            window.addEventListener('mousemove', handler, false);

            window.addEventListener('mouseup', () => {
                this._currentlyDragged = false;
                window.removeEventListener('mousemove', handler, false);
            }, { once: true });
        });

        pinVolume.addEventListener("touchstart", (event) => {
            if (event.changedTouches.length == 1 && event.targetTouches.length ==1) {
                let touch = event.changedTouches[0];
                this._currentlyDragged = touch.target;
                let touchId = touch.identifier;

                let myTouch = (event) => {
                    for (let i = 0; i< event.changedTouches.length; i++) {
                        let t = event.changedTouches.item(i);
                        if (t.identifier === touch.identifier) return t;
                    }
                }
                
                let handler = (event) => {
                    let t = myTouch(event);
                    if (t) {
                    let evt = touchToEvent(t, "mousemove");
                    this._onChangeVolume(evt);
                    }
                }
                window.addEventListener("touchmove", handler);
                window.addEventListener("touchend", (event) => {
                    let t = myTouch(event);
                    if (t) {
                    this._currentlyDragged = false;
                    window.removeEventListener("touchmove", handler);
                    }

                }, {once:true});
            }
        }, {passive:true});

        sliderTime.addEventListener('click', (evt) => {
            if (!this._currentlyDragged) this._onMoveSlider(evt, true);
        });

        sliderVolume.addEventListener('click', this._onChangeVolume.bind(this));

        this._playpauseBtn.addEventListener('click', this.togglePlay.bind(this));
        volumeBtn.addEventListener('click', () => {
            volumeBtn.classList.toggle('open');
            volumeControls.classList.toggle('hidden');
        }
        );

        this.initPlayer();
    }

    initPlayer() {
        this._player.addEventListener('timeupdate', this._updateProgress.bind(this));
        this._player.addEventListener('volumechange', this._updateVolume.bind(this));
        this._player.addEventListener('loadedmetadata', this._updateTotal.bind(this));
        this._player.addEventListener('canplay', () => {
            //console.log("Can play");
            this._showPlay();
        });
        this._player.addEventListener('ended', () => {
            this._playPause.attributes.d.value = PLAY;
            let event = new Event("ended");
            this._rootElem.dispatchEvent(event);
            console.log("Track ended");
        });
        let state = this._player.readyState;
        if (state > 1) this._updateTotal();
        if (state > 2) this._showPlay();

    }

    _updateTotal() {
        this._totalTime.textContent = formatTime(this.getTotalTime());
    }

    _updateProgress() {
        let event = new CustomEvent('timeupdate', {detail:{
            currentTime: this._player.currentTime,
            totalTime: this._player.duration
        }});
        this._rootElem.dispatchEvent(event);
        if (!this._currentlyDragged) {
            let current = this._player.currentTime;
            let percent = (current / this.getTotalTime()) * 100;
            if (percent > 100) percent = 100;
            if (isNaN(percent)) percent = 0;
            this._progress.style.width = percent + '%';
            this._currentTime.textContent = formatTime(current);
        }
    }

    _updateVolume() {
        this._volumeProgress.style.height = this._player.volume * 100 + '%';
        if (this._player.volume >= 0.5) {
            this._speaker.attributes.d.value = VOLUME_FULL;
        } else if (this._player.volume < 0.5 && this._player.volume > 0.05) {
            this._speaker.attributes.d.value = VOLUME_MED;
        } else if (this._player.volume <= 0.05) {
            this._speaker.attributes.d.value = VOLUME_LOW;
        }
    }

    _getRangeBox(event) {
        let rangeBox = event.target;
        let el = this._currentlyDragged;
        if (event.type == 'click' && event.target.classList.contains('pin')) {
            rangeBox = event.target.parentElement.parentElement;
        }
        if (event.type == 'mousemove' || event.type == 'mouseup') {
            rangeBox = el.parentElement.parentElement;
        }
        return rangeBox;
    }

    _getCoefficient(event) {
        let slider = this._getRangeBox(event);
        let rect = slider.getBoundingClientRect();
        let K = 0;
        if (slider.dataset.direction == 'horizontal') {

            let offsetX = event.clientX - slider.offsetLeft;
            let width = slider.clientWidth;
            K = offsetX / width;
            K = K < 0 ? 0 : K > 1 ? 1 : K;

        } else if (slider.dataset.direction == 'vertical') {

            let height = slider.clientHeight;
            let offsetY = event.clientY - rect.top;
            K = 1 - offsetY / height;
            K = K < 0 ? 0 : K > 1 ? 1 : K;

        }
        return K;
    }

    getTotalTime() {
        if (this.unsized && this.knownDuration) {
            return this.knownDuration
        } else {
            return this._player.duration;
        }
    }

    _onMoveSlider(event, jump = false) {

        let k = this._getCoefficient(event);
        let currentTime = this.getTotalTime() * k;
        let percent = k * 100;
        this._progress.style.width = percent + '%';
        this._currentTime.textContent = formatTime(currentTime);
        if (jump) {
            this.jumpToTime(currentTime);
        }
    }

    _onChangeVolume(event) {
        this._player.volume = this._getCoefficient(event);

    }


    _showPlay() {
        this._playpauseBtn.style.display = 'block';
        this._loading.style.display = 'none';
    }

    _hidePlay() {
        this._playpauseBtn.style.display = 'none';
        this._loading.style.display = 'show';
    }

    jumpToTime(time) {
        console.log(`Jumping to time ${time}, duration: ${this._player.duration}`);
        if (this._player.buffered.length) console.log(`${this._player.buffered.length} buffered: ${formatTime(this._player.buffered.start(0))} - ${formatTime(this._player.buffered.end(0))}`);
        if (Math.abs(time - this._player.currentTime) > 1 && isFinite(time)) {
            this._player.currentTime = time;
        }
    }

    togglePlay() {
        if (this._player.paused) {
            this.play();
        } else {
            this.pause();
        }
    }

    setUrl(url, options) {
        if (options && "duration" in options) {
            this.knownDuration = options.duration;
        } else {
            this.knownDuration = null;
        }
        if (options && options.transcoded) this.unsized = true
        else if (options && options.unsized) this.unsized = true
        else this.unsized = false;
        if (!url) {
            this._player.src = "";
            this._updateTotal();
            this._updateProgress();
            this._hidePlay();
            this._loading.style.display = 'none';
        } else {
            this._player.src = url;
            this._hidePlay();
        }
    }

    play() { 
        this._playPause.attributes.d.value = PAUSE;
        return this._player.play()
        .catch((e) => {
            this.pause();
            console.log("Play error",e);
        }
        );
    }

    pause() {
        this._playPause.attributes.d.value = PLAY;
        this._player.pause();
        
    }
}