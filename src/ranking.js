/**
 * Photo Tinder Desktop - Ranking Mode
 * Ported to Tauri from FastAPI web app
 */

// Tauri API - use existing globals from swipe.js if available
// (both scripts share the same scope)

// DOM Elements
let triageMode, rankingMode, rankingInitOverlay, leaderboardModal;
let triageModeBtn, rankingModeBtn;
let leftPanel, rightPanel, leftPhoto, rightPhoto;
let leftMu, leftSigma, rightMu, rightSigma;
let comparisonsCount, photosRanked, rankingPhase;
let rankingDoneMessage;
let leftWinsBtn, tieBtn, rightWinsBtn, skipCompareBtn;
let rescanBtn, leaderboardBtn, closeLeaderboard, leaderboardGrid;

// State
let currentLeftId = null;
let currentRightId = null;
let rankingInitialized = false;

/**
 * Initialize ranking mode handlers
 */
function initRanking() {
    try {
        // Get DOM elements
        triageMode = document.getElementById('triageMode');
        rankingMode = document.getElementById('rankingMode');
        rankingInitOverlay = document.getElementById('rankingInitOverlay');
        leaderboardModal = document.getElementById('leaderboardModal');

        triageModeBtn = document.getElementById('triageModeBtn');
        rankingModeBtn = document.getElementById('rankingModeBtn');

        leftPanel = document.getElementById('leftPanel');
        rightPanel = document.getElementById('rightPanel');
        leftPhoto = document.getElementById('leftPhoto');
        rightPhoto = document.getElementById('rightPhoto');

        leftMu = document.getElementById('leftMu');
        leftSigma = document.getElementById('leftSigma');
        rightMu = document.getElementById('rightMu');
        rightSigma = document.getElementById('rightSigma');

        comparisonsCount = document.getElementById('comparisonsCount');
        photosRanked = document.getElementById('photosRanked');
        rankingPhase = document.getElementById('rankingPhase');
        rankingDoneMessage = document.getElementById('rankingDoneMessage');

        leftWinsBtn = document.getElementById('leftWinsBtn');
        tieBtn = document.getElementById('tieBtn');
        rightWinsBtn = document.getElementById('rightWinsBtn');
        skipCompareBtn = document.getElementById('skipCompareBtn');

        rescanBtn = document.getElementById('rescanBtn');
        leaderboardBtn = document.getElementById('leaderboardBtn');
        closeLeaderboard = document.getElementById('closeLeaderboard');
        leaderboardGrid = document.getElementById('leaderboardGrid');

        if (!rankingModeBtn) {
            console.error('rankingModeBtn not found!');
            return;
        }

        // Mode toggle
        triageModeBtn.addEventListener('click', () => switchMode('triage'));
        rankingModeBtn.addEventListener('click', () => switchMode('ranking'));

    // Ranking buttons
    leftWinsBtn.addEventListener('click', () => submitComparison('left'));
    tieBtn.addEventListener('click', () => submitComparison('tie'));
    rightWinsBtn.addEventListener('click', () => submitComparison('right'));
    skipCompareBtn.addEventListener('click', () => submitComparison('skip'));

    // Click on photo panels
    leftPanel.addEventListener('click', () => {
        if (currentLeftId) submitComparison('left');
    });
    rightPanel.addEventListener('click', () => {
        if (currentRightId) submitComparison('right');
    });

    // Rescan and leaderboard
    rescanBtn.addEventListener('click', rescanPhotos);
    leaderboardBtn.addEventListener('click', showLeaderboard);
    closeLeaderboard.addEventListener('click', () => {
        leaderboardModal.style.display = 'none';
    });
    leaderboardModal.addEventListener('click', (e) => {
        if (e.target === leaderboardModal) {
            leaderboardModal.style.display = 'none';
        }
    });

    // Keyboard shortcuts for ranking
    document.addEventListener('keydown', onRankingKeyDown);

    // Check initial mode
    checkMode();
    } catch (e) {
        console.error('Error in initRanking:', e);
    }
}

/**
 * Check and set initial mode from backend
 */
async function checkMode() {
    try {
        const mode = await invoke('get_mode');
        if (mode === 'ranking') {
            switchMode('ranking');
        }
    } catch (e) {
        console.error('Error checking mode:', e);
    }
}

/**
 * Switch between triage and ranking modes
 */
async function switchMode(mode) {
    try {
        await invoke('set_mode', { mode });

        if (mode === 'triage') {
            triageMode.style.display = 'block';
            rankingMode.style.display = 'none';
            triageModeBtn.classList.add('active');
            rankingModeBtn.classList.remove('active');
        } else {
            triageMode.style.display = 'none';
            rankingMode.style.display = 'block';
            triageModeBtn.classList.remove('active');
            rankingModeBtn.classList.add('active');

            // Check if ranking is initialized
            const stats = await invoke('get_ranking_stats');
            if (!stats.initialized) {
                await initializeRanking();
            } else {
                rankingInitialized = true;
                updateRankingStats(stats);
                await loadNextPair();
            }
        }
    } catch (e) {
        console.error('Error switching mode:', e);
    }
}

/**
 * Initialize ranking mode (first time)
 */
async function initializeRanking() {
    rankingInitOverlay.style.display = 'flex';
    document.getElementById('initStatus').textContent = 'Scanning accepted photos and computing hashes...';

    try {
        const result = await invoke('init_ranking');
        rankingInitialized = true;
        rankingInitOverlay.style.display = 'none';

        updateRankingStats(result);
        await loadNextPair();
    } catch (e) {
        console.error('Error initializing ranking:', e);
        document.getElementById('initStatus').textContent = 'Error: ' + e;
        setTimeout(() => {
            rankingInitOverlay.style.display = 'none';
        }, 3000);
    }
}

/**
 * Update ranking stats display
 */
function updateRankingStats(stats) {
    comparisonsCount.textContent = `${stats.total_comparisons} comparisons`;
    photosRanked.textContent = `${stats.total_photos} photos`;
    rankingPhase.textContent = `Phase: ${stats.phase}`;
}

/**
 * Load next pair for comparison
 */
async function loadNextPair() {
    try {
        const data = await invoke('get_pair');

        if (data.error) {
            console.error('Error getting pair:', data.message);
            return;
        }

        if (data.done) {
            showRankingDone();
            return;
        }

        // Hide done message
        rankingDoneMessage.style.display = 'none';
        document.querySelector('.comparison-container').style.display = 'flex';
        document.querySelector('.ranking-buttons').style.display = 'flex';

        // Update current IDs
        currentLeftId = data.left.id;
        currentRightId = data.right.id;

        // Load images
        leftPhoto.src = convertFileSrc(data.left.file_path);
        rightPhoto.src = convertFileSrc(data.right.file_path);

        // Update scores
        leftMu.textContent = Math.round(data.left.mu);
        leftSigma.textContent = Math.round(data.left.sigma);
        rightMu.textContent = Math.round(data.right.mu);
        rightSigma.textContent = Math.round(data.right.sigma);

        // Update stats
        if (data.stats) {
            updateRankingStats(data.stats);
        }
    } catch (e) {
        console.error('Error loading pair:', e);
    }
}

/**
 * Show ranking done message
 */
function showRankingDone() {
    rankingDoneMessage.style.display = 'flex';
    document.querySelector('.comparison-container').style.display = 'none';
    document.querySelector('.ranking-buttons').style.display = 'none';
}

/**
 * Submit comparison result
 */
async function submitComparison(result) {
    if (!currentLeftId || !currentRightId) return;

    try {
        await invoke('compare', {
            leftId: currentLeftId,
            rightId: currentRightId,
            result: result
        });

        await loadNextPair();
    } catch (e) {
        console.error('Error submitting comparison:', e);
    }
}

/**
 * Rescan for new photos
 */
async function rescanPhotos() {
    rankingInitOverlay.style.display = 'flex';
    document.getElementById('initStatus').textContent = 'Rescanning for new photos...';

    try {
        // Re-initialize ranking (will add new photos)
        const result = await invoke('init_ranking');
        rankingInitOverlay.style.display = 'none';
        updateRankingStats(result);
        await loadNextPair();
    } catch (e) {
        console.error('Error rescanning:', e);
        document.getElementById('initStatus').textContent = 'Error: ' + e;
        setTimeout(() => {
            rankingInitOverlay.style.display = 'none';
        }, 3000);
    }
}

/**
 * Show leaderboard
 */
async function showLeaderboard() {
    try {
        const photos = await invoke('get_leaderboard', { limit: 50 });

        leaderboardGrid.innerHTML = '';

        photos.forEach((photo, idx) => {
            const item = document.createElement('div');
            item.className = 'leaderboard-item';
            item.innerHTML = `
                <span class="rank">#${idx + 1}</span>
                <img src="${convertFileSrc(photo.file_path)}" alt="Photo ${idx + 1}">
                <div class="leaderboard-score">
                    <span class="score">${Math.round(photo.score)}</span>
                    <span class="mu-sigma">${Math.round(photo.mu)} ± ${Math.round(photo.sigma)}</span>
                </div>
            `;

            // Click to open in lightbox
            item.querySelector('img').addEventListener('click', () => {
                if (typeof openLightbox === 'function') {
                    openLightbox(convertFileSrc(photo.file_path));
                }
            });

            leaderboardGrid.appendChild(item);
        });

        leaderboardModal.style.display = 'flex';
    } catch (e) {
        console.error('Error loading leaderboard:', e);
    }
}

/**
 * Keyboard handler for ranking mode
 */
function onRankingKeyDown(e) {
    // Only handle when ranking mode is visible
    if (rankingMode.style.display === 'none') return;

    switch (e.key.toLowerCase()) {
        case 'a':
            e.preventDefault();
            submitComparison('left');
            break;
        case 's':
            e.preventDefault();
            submitComparison('tie');
            break;
        case 'd':
            e.preventDefault();
            submitComparison('right');
            break;
        case 'w':
            e.preventDefault();
            submitComparison('skip');
            break;
        case 'u':
            e.preventDefault();
            triggerUndo();
            break;
    }
}

// Initialize ranking mode when DOM is ready
document.addEventListener('DOMContentLoaded', initRanking);
