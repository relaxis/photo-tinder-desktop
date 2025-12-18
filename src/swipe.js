/**
 * Photo Tinder Desktop - Swipe handling and UI logic with Lightbox
 * Ported to Tauri from FastAPI web app
 */

// Tauri API - make globally available for all scripts
var invoke = window.__TAURI__.core.invoke;
var convertFileSrc = window.__TAURI__.core.convertFileSrc;

// State
let currentImageId = null;
let currentFilePath = null;
let isAnimating = false;
const preloadCache = new Map();

// DOM Elements (initialized in DOMContentLoaded)
let swipeCard, currentImage, doneMessage, filename, sourceFolder;
let progressFill, progressText, acceptedCount, rejectedCount, skippedCount;
let acceptBtn, rejectBtn, skipBtn, undoBtn;
let lightbox, lightboxContainer, lightboxImage, lightboxClose;
let setupScreen, mainApp;

// Touch/mouse state for swipe
let startX = 0;
let startY = 0;
let currentX = 0;
let currentY = 0;
let isDragging = false;
let touchStartTime = 0;

// Thresholds
const SWIPE_THRESHOLD = 80;
const SWIPE_THRESHOLD_Y = 60;
const TAP_THRESHOLD = 15;
const TAP_TIME_THRESHOLD = 300;

// Lightbox zoom state
let lightboxOpen = false;
let scale = 1;
let panX = 0;
let panY = 0;
let lastScale = 1;
let lastPanX = 0;
let lastPanY = 0;
let initialPinchDistance = 0;
let initialPanX = 0;
let initialPanY = 0;
let isPinching = false;
let isPanning = false;
let panStartX = 0;
let panStartY = 0;

// Settings state
let folderToDelete = null;
let browserTarget = null;
let browserCurrentPath = '';

// Setup state
let setupConfig = {
    source_folders: [],
    accepted_folder: '',
    rejected_folder: ''
};

/**
 * Initialize the app
 */
async function init() {
    // Get DOM elements
    swipeCard = document.getElementById('swipeCard');
    currentImage = document.getElementById('currentImage');
    doneMessage = document.getElementById('doneMessage');
    filename = document.getElementById('filename');
    sourceFolder = document.getElementById('sourceFolder');
    progressFill = document.getElementById('progressFill');
    progressText = document.getElementById('progressText');
    acceptedCount = document.getElementById('acceptedCount');
    rejectedCount = document.getElementById('rejectedCount');
    skippedCount = document.getElementById('skippedCount');
    acceptBtn = document.getElementById('acceptBtn');
    rejectBtn = document.getElementById('rejectBtn');
    skipBtn = document.getElementById('skipBtn');
    undoBtn = document.getElementById('undoBtn');
    lightbox = document.getElementById('lightbox');
    lightboxContainer = document.getElementById('lightboxContainer');
    lightboxImage = document.getElementById('lightboxImage');
    lightboxClose = document.getElementById('lightboxClose');
    setupScreen = document.getElementById('setupScreen');
    mainApp = document.getElementById('mainApp');

    // Check if config is valid
    const isValid = await invoke('is_config_valid');

    if (isValid) {
        // Config exists, initialize app
        await invoke('initialize_app');
        showMainApp();
        await loadCurrentImage();
    } else {
        // Show setup screen
        showSetupScreen();
    }

    bindEvents();
    initSettings();
    initSetup();
}

/**
 * Show setup screen
 */
function showSetupScreen() {
    setupScreen.style.display = 'flex';
    mainApp.style.display = 'none';
}

/**
 * Show main app
 */
function showMainApp() {
    setupScreen.style.display = 'none';
    mainApp.style.display = 'block';
}

/**
 * Initialize setup screen handlers
 */
async function initSetup() {
    // Load existing config if any
    const config = await invoke('get_config');
    setupConfig = {
        source_folders: config.source_folders || [],
        accepted_folder: config.accepted_folder || '',
        rejected_folder: config.rejected_folder || ''
    };

    updateSetupUI();

    // Add source folder button
    document.getElementById('setupAddSourceBtn').addEventListener('click', () => {
        browserTarget = 'setup_source';
        openBrowser();
    });

    // Browse buttons for destinations
    document.querySelectorAll('#setupScreen .setup-btn.browse').forEach(btn => {
        btn.addEventListener('click', () => {
            browserTarget = 'setup_' + btn.dataset.target;
            openBrowser();
        });
    });

    // Start button
    document.getElementById('setupStartBtn').addEventListener('click', startApp);
}

/**
 * Update setup UI with current config
 */
function updateSetupUI() {
    // Source folders list
    const sourceList = document.getElementById('setupSourceFolders');
    sourceList.innerHTML = '';

    if (setupConfig.source_folders.length === 0) {
        sourceList.innerHTML = '<div class="setup-empty">No source folders added yet</div>';
    } else {
        setupConfig.source_folders.forEach((folder, idx) => {
            const item = document.createElement('div');
            item.className = 'setup-folder-item';
            item.innerHTML = `
                <span class="folder-path" title="${folder}">${folder}</span>
                <button class="remove-btn" data-idx="${idx}">&times;</button>
            `;
            sourceList.appendChild(item);
        });

        // Bind remove buttons
        sourceList.querySelectorAll('.remove-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                const idx = parseInt(btn.dataset.idx);
                setupConfig.source_folders.splice(idx, 1);
                updateSetupUI();
            });
        });
    }

    // Destination folders
    document.getElementById('setupAcceptedDisplay').textContent = setupConfig.accepted_folder || 'Not set';
    document.getElementById('setupRejectedDisplay').textContent = setupConfig.rejected_folder || 'Not set';

    // Enable/disable start button
    const canStart = setupConfig.source_folders.length > 0 &&
                     setupConfig.accepted_folder &&
                     setupConfig.rejected_folder;
    document.getElementById('setupStartBtn').disabled = !canStart;
}

/**
 * Start the app after setup
 */
async function startApp() {
    try {
        await invoke('save_config', { config: setupConfig });
        await invoke('initialize_app');
        showMainApp();
        await loadCurrentImage();
    } catch (error) {
        console.error('Error starting app:', error);
        alert('Failed to start: ' + error);
    }
}

/**
 * Load current image from backend
 */
async function loadCurrentImage() {
    try {
        const data = await invoke('get_current_image');

        if (data.done) {
            showDoneMessage(data);
            return;
        }

        currentImageId = data.id;
        currentFilePath = data.file_path;
        filename.textContent = data.filename;
        sourceFolder.textContent = data.source_folder;

        // Update progress
        updateStats(data.stats);
        const progress = ((data.stats.processed) / data.stats.total) * 100;
        progressFill.style.width = `${progress}%`;
        progressText.textContent = `${data.index + 1} / ${data.total_pending} pending`;

        // Load image using Tauri's asset protocol
        const imgUrl = convertFileSrc(data.file_path);

        // Check preload cache
        if (preloadCache.has(data.id)) {
            currentImage.src = preloadCache.get(data.id).src;
        } else {
            currentImage.src = imgUrl;
        }

        // Show card
        swipeCard.style.display = 'block';
        doneMessage.style.display = 'none';
        resetCardPosition();

        // Preload next images
        preloadNextImages();

    } catch (error) {
        console.error('Error loading image:', error);
        filename.textContent = 'Error loading image';
    }
}

/**
 * Update statistics display
 */
function updateStats(stats) {
    acceptedCount.textContent = stats.accepted;
    rejectedCount.textContent = stats.rejected;
    skippedCount.textContent = stats.skipped;
}

/**
 * Show done message
 */
function showDoneMessage(data) {
    swipeCard.style.display = 'none';
    doneMessage.style.display = 'flex';
    updateStats(data.stats);
    progressFill.style.width = '100%';
    progressText.textContent = 'Complete!';
}

/**
 * Preload next few images
 */
async function preloadNextImages() {
    try {
        const paths = await invoke('get_preload_list');

        for (const filePath of paths) {
            const id = filePath; // Use path as cache key
            if (!preloadCache.has(id)) {
                const img = new Image();
                img.src = convertFileSrc(filePath);
                preloadCache.set(id, img);
            }
        }

        // Clean old cache entries
        if (preloadCache.size > 15) {
            const keys = [...preloadCache.keys()];
            for (let i = 0; i < keys.length - 10; i++) {
                preloadCache.delete(keys[i]);
            }
        }
    } catch (error) {
        console.error('Error preloading:', error);
    }
}

/**
 * Bind event listeners
 */
function bindEvents() {
    // Touch events for swipe card
    swipeCard.addEventListener('touchstart', onTouchStart, { passive: true });
    swipeCard.addEventListener('touchmove', onTouchMove, { passive: false });
    swipeCard.addEventListener('touchend', onTouchEnd);

    // Mouse events for swipe card
    swipeCard.addEventListener('mousedown', onMouseDown);
    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);

    // Keyboard events
    document.addEventListener('keydown', onKeyDown);

    // Button events
    acceptBtn.addEventListener('click', () => triggerSwipe('right'));
    rejectBtn.addEventListener('click', () => triggerSwipe('left'));
    skipBtn.addEventListener('click', () => triggerSwipe('down'));
    undoBtn.addEventListener('click', triggerUndo);

    // Refresh button
    document.getElementById('refreshFoldersBtn').addEventListener('click', refreshFolders);

    // Mode toggle buttons
    const triageModeBtn = document.getElementById('triageModeBtn');
    const rankingModeBtn = document.getElementById('rankingModeBtn');
    if (triageModeBtn && rankingModeBtn) {
        triageModeBtn.addEventListener('click', () => {
            if (typeof switchMode === 'function') switchMode('triage');
        });
        rankingModeBtn.addEventListener('click', () => {
            if (typeof switchMode === 'function') switchMode('ranking');
        });
    }

    // Lightbox events
    lightboxClose.addEventListener('click', closeLightbox);
    lightboxClose.addEventListener('touchend', (e) => {
        e.preventDefault();
        closeLightbox();
    });
    lightbox.addEventListener('click', (e) => {
        if (e.target === lightbox || e.target === lightboxContainer) {
            closeLightbox();
        }
    });
    lightboxContainer.addEventListener('touchstart', onLightboxTouchStart, { passive: false });
    lightboxContainer.addEventListener('touchmove', onLightboxTouchMove, { passive: false });
    lightboxContainer.addEventListener('touchend', onLightboxTouchEnd);
    lightboxContainer.addEventListener('wheel', onLightboxWheel, { passive: false });
    lightboxContainer.addEventListener('dblclick', onLightboxDoubleClick);
}

// Touch event handlers for swipe card
function onTouchStart(e) {
    if (isAnimating || lightboxOpen) return;
    const touch = e.touches[0];
    startX = touch.clientX;
    startY = touch.clientY;
    touchStartTime = Date.now();
    isDragging = true;
    swipeCard.classList.add('swiping');
}

function onTouchMove(e) {
    if (!isDragging || lightboxOpen) return;
    e.preventDefault();
    const touch = e.touches[0];
    currentX = touch.clientX - startX;
    currentY = touch.clientY - startY;
    updateCardPosition();
}

function onTouchEnd(e) {
    if (!isDragging || lightboxOpen) return;
    isDragging = false;
    swipeCard.classList.remove('swiping');

    const touchDuration = Date.now() - touchStartTime;
    const totalMovement = Math.sqrt(currentX * currentX + currentY * currentY);

    if (touchDuration < TAP_TIME_THRESHOLD && totalMovement < TAP_THRESHOLD) {
        openLightbox();
        currentX = 0;
        currentY = 0;
        resetCardPosition();
        return;
    }

    handleSwipeEnd();
}

// Mouse event handlers
function onMouseDown(e) {
    if (isAnimating || lightboxOpen) return;
    startX = e.clientX;
    startY = e.clientY;
    touchStartTime = Date.now();
    isDragging = true;
    swipeCard.classList.add('swiping');
}

function onMouseMove(e) {
    if (!isDragging || lightboxOpen) return;
    currentX = e.clientX - startX;
    currentY = e.clientY - startY;
    updateCardPosition();
}

function onMouseUp(e) {
    if (!isDragging || lightboxOpen) return;
    isDragging = false;
    swipeCard.classList.remove('swiping');

    const touchDuration = Date.now() - touchStartTime;
    const totalMovement = Math.sqrt(currentX * currentX + currentY * currentY);

    if (touchDuration < TAP_TIME_THRESHOLD && totalMovement < TAP_THRESHOLD) {
        openLightbox();
        currentX = 0;
        currentY = 0;
        resetCardPosition();
        return;
    }

    handleSwipeEnd();
}

/**
 * Keyboard handler
 */
function onKeyDown(e) {
    if (e.key === 'Escape' && lightboxOpen) {
        closeLightbox();
        return;
    }

    if (isAnimating || lightboxOpen) return;

    switch (e.key) {
        case 'ArrowRight':
            e.preventDefault();
            triggerSwipe('right');
            break;
        case 'ArrowLeft':
            e.preventDefault();
            triggerSwipe('left');
            break;
        case 'ArrowDown':
        case 's':
        case 'S':
            e.preventDefault();
            triggerSwipe('down');
            break;
        case 'ArrowUp':
        case 'u':
        case 'U':
            e.preventDefault();
            triggerUndo();
            break;
    }
}

function updateCardPosition() {
    const rotation = currentX * 0.05;
    swipeCard.style.transform = `translate(${currentX}px, ${currentY}px) rotate(${rotation}deg)`;

    swipeCard.classList.remove('swipe-left', 'swipe-right', 'swipe-down');

    if (currentX > 40) {
        swipeCard.classList.add('swipe-right');
    } else if (currentX < -40) {
        swipeCard.classList.add('swipe-left');
    } else if (currentY > 40) {
        swipeCard.classList.add('swipe-down');
    }
}

function handleSwipeEnd() {
    if (currentX > SWIPE_THRESHOLD) {
        triggerSwipe('right');
    } else if (currentX < -SWIPE_THRESHOLD) {
        triggerSwipe('left');
    } else if (currentY > SWIPE_THRESHOLD_Y) {
        triggerSwipe('down');
    } else {
        resetCardPosition();
    }

    currentX = 0;
    currentY = 0;
}

function resetCardPosition() {
    swipeCard.style.transition = 'transform 0.3s ease-out';
    swipeCard.style.transform = 'translate(0, 0) rotate(0deg)';
    swipeCard.classList.remove('swipe-left', 'swipe-right', 'swipe-down');

    setTimeout(() => {
        swipeCard.style.transition = '';
    }, 300);
}

/**
 * Trigger a swipe action
 */
async function triggerSwipe(direction) {
    if (isAnimating || !currentImageId) return;

    isAnimating = true;

    // Animate out
    const translateX = direction === 'right' ? '150%' : direction === 'left' ? '-150%' : '0';
    const translateY = direction === 'down' ? '150%' : '0';
    const rotation = direction === 'right' ? 30 : direction === 'left' ? -30 : 0;

    swipeCard.style.transition = 'transform 0.3s ease-out, opacity 0.3s ease-out';
    swipeCard.style.transform = `translate(${translateX}, ${translateY}) rotate(${rotation}deg)`;
    swipeCard.style.opacity = '0';

    try {
        const data = await invoke('swipe', {
            imageId: currentImageId,
            direction: direction
        });

        if (!data.success) {
            console.error('Swipe failed:', data);
        }

        await new Promise(resolve => setTimeout(resolve, 300));

        swipeCard.style.transition = '';
        swipeCard.style.opacity = '1';
        resetCardPosition();

        await loadCurrentImage();

    } catch (error) {
        console.error('Error during swipe:', error);
        resetCardPosition();
        swipeCard.style.opacity = '1';
    }

    isAnimating = false;
}

/**
 * Refresh folders - rescan all source folders
 */
async function refreshFolders() {
    try {
        await invoke('initialize_app');
        await loadCurrentImage();
    } catch (error) {
        console.error('Error refreshing folders:', error);
    }
}

/**
 * Trigger undo action
 */
async function triggerUndo() {
    if (isAnimating) return;

    const rankingModeEl = document.getElementById('rankingMode');
    const isRankingMode = rankingModeEl && rankingModeEl.style.display !== 'none';

    if (isRankingMode) {
        try {
            const data = await invoke('undo_ranking');
            if (data.success && typeof loadNextPair === 'function') {
                await loadNextPair();
            } else {
                console.log('Ranking undo:', data.message);
            }
        } catch (error) {
            console.error('Error during ranking undo:', error);
        }
    } else {
        try {
            const data = await invoke('undo');
            if (data.success) {
                await loadCurrentImage();
            } else {
                console.log('Undo:', data.message);
            }
        } catch (error) {
            console.error('Error during undo:', error);
        }
    }
}

// ==================== LIGHTBOX FUNCTIONS ====================

function openLightbox(imageUrl) {
    if (imageUrl) {
        lightboxImage.src = imageUrl;
    } else if (currentFilePath) {
        lightboxImage.src = convertFileSrc(currentFilePath);
    } else {
        return;
    }

    lightbox.classList.add('active');
    lightboxOpen = true;

    scale = 1;
    panX = 0;
    panY = 0;
    updateLightboxTransform();

    document.body.style.overflow = 'hidden';
}

function closeLightbox() {
    lightbox.classList.remove('active');
    lightbox.classList.remove('zoomed');
    lightboxOpen = false;
    document.body.style.overflow = '';
    scale = 1;
    panX = 0;
    panY = 0;
}

function updateLightboxTransform() {
    lightboxImage.style.transform = `translate(${panX}px, ${panY}px) scale(${scale})`;

    if (scale > 1.05) {
        lightbox.classList.add('zoomed');
        lightboxContainer.classList.add('panning');
    } else {
        lightbox.classList.remove('zoomed');
        lightboxContainer.classList.remove('panning');
    }
}

function getTouchDistance(touches) {
    const dx = touches[0].clientX - touches[1].clientX;
    const dy = touches[0].clientY - touches[1].clientY;
    return Math.sqrt(dx * dx + dy * dy);
}

function getTouchCenter(touches) {
    return {
        x: (touches[0].clientX + touches[1].clientX) / 2,
        y: (touches[0].clientY + touches[1].clientY) / 2
    };
}

function onLightboxTouchStart(e) {
    if (e.touches.length === 2) {
        e.preventDefault();
        isPinching = true;
        isPanning = false;
        initialPinchDistance = getTouchDistance(e.touches);
        lastScale = scale;
        initialPanX = panX;
        initialPanY = panY;
    } else if (e.touches.length === 1 && scale > 1) {
        e.preventDefault();
        isPanning = true;
        isPinching = false;
        panStartX = e.touches[0].clientX - panX;
        panStartY = e.touches[0].clientY - panY;
        lightboxContainer.classList.add('panning');
    }
}

function onLightboxTouchMove(e) {
    if (isPinching && e.touches.length === 2) {
        e.preventDefault();
        const currentDistance = getTouchDistance(e.touches);
        const scaleChange = currentDistance / initialPinchDistance;
        scale = Math.max(0.5, Math.min(5, lastScale * scaleChange));
        updateLightboxTransform();
    } else if (isPanning && e.touches.length === 1 && scale > 1) {
        e.preventDefault();
        panX = e.touches[0].clientX - panStartX;
        panY = e.touches[0].clientY - panStartY;
        const maxPan = (scale - 1) * 200;
        panX = Math.max(-maxPan, Math.min(maxPan, panX));
        panY = Math.max(-maxPan, Math.min(maxPan, panY));
        updateLightboxTransform();
    }
}

function onLightboxTouchEnd(e) {
    if (e.touches.length === 0) {
        if (!isPinching && !isPanning && scale <= 1.05) {
            closeLightbox();
        }
        isPinching = false;
        isPanning = false;
        lightboxContainer.classList.remove('panning');

        if (scale < 1.05 && scale > 0.95) {
            scale = 1;
            panX = 0;
            panY = 0;
            updateLightboxTransform();
        }
    } else if (e.touches.length === 1 && isPinching) {
        isPinching = false;
        if (scale > 1) {
            isPanning = true;
            panStartX = e.touches[0].clientX - panX;
            panStartY = e.touches[0].clientY - panY;
        }
    }
}

function onLightboxWheel(e) {
    e.preventDefault();
    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    scale = Math.max(0.5, Math.min(5, scale * delta));

    if (scale < 1.05) {
        scale = 1;
        panX = 0;
        panY = 0;
    }

    updateLightboxTransform();
}

function onLightboxDoubleClick(e) {
    if (scale > 1.05) {
        scale = 1;
        panX = 0;
        panY = 0;
    } else {
        scale = 2;
        const rect = lightboxContainer.getBoundingClientRect();
        const centerX = rect.width / 2;
        const centerY = rect.height / 2;
        panX = (centerX - e.clientX) * 0.5;
        panY = (centerY - e.clientY) * 0.5;
    }
    updateLightboxTransform();
}

// ==================== SETTINGS & FOLDER BROWSER ====================

let settingsModal, folderBrowserModal, confirmDeleteModal;

function initSettings() {
    settingsModal = document.getElementById('settingsModal');
    folderBrowserModal = document.getElementById('folderBrowserModal');
    confirmDeleteModal = document.getElementById('confirmDeleteModal');

    document.getElementById('settingsBtn').addEventListener('click', openSettings);
    document.getElementById('closeSettings').addEventListener('click', closeSettings);
    settingsModal.addEventListener('click', (e) => {
        if (e.target === settingsModal) closeSettings();
    });

    document.querySelectorAll('.browse-btn').forEach(btn => {
        btn.addEventListener('click', () => openBrowser(btn.dataset.target));
    });
    document.getElementById('addSourceFolderBtn').addEventListener('click', () => openBrowser('source'));

    document.getElementById('closeBrowser').addEventListener('click', closeBrowser);
    document.getElementById('browserCancelBtn').addEventListener('click', closeBrowser);
    document.getElementById('browserSelectBtn').addEventListener('click', selectFolder);
    document.getElementById('browserUpBtn').addEventListener('click', navigateUp);
    folderBrowserModal.addEventListener('click', (e) => {
        if (e.target === folderBrowserModal) closeBrowser();
    });

    document.getElementById('keepDecisionsBtn').addEventListener('click', () => deleteFolder(false));
    document.getElementById('clearDecisionsBtn').addEventListener('click', () => deleteFolder(true));
    document.getElementById('cancelDeleteBtn').addEventListener('click', () => {
        folderToDelete = null;
        confirmDeleteModal.style.display = 'none';
    });
    confirmDeleteModal.addEventListener('click', (e) => {
        if (e.target === confirmDeleteModal) {
            folderToDelete = null;
            confirmDeleteModal.style.display = 'none';
        }
    });
}

async function openSettings() {
    await loadFolders();
    settingsModal.style.display = 'flex';
}

function closeSettings() {
    settingsModal.style.display = 'none';
}

async function loadFolders() {
    try {
        const data = await invoke('get_folders');

        const acceptedDisplay = document.getElementById('acceptedFolderDisplay');
        const rejectedDisplay = document.getElementById('rejectedFolderDisplay');

        acceptedDisplay.textContent = data.accepted_folder || 'Not set';
        acceptedDisplay.title = data.accepted_folder || '';
        rejectedDisplay.textContent = data.rejected_folder || 'Not set';
        rejectedDisplay.title = data.rejected_folder || '';

        const foldersList = document.getElementById('foldersList');
        foldersList.innerHTML = '';

        data.folders.forEach(folder => {
            const item = document.createElement('div');
            item.className = 'folder-item' + (folder.exists ? '' : ' missing');
            item.innerHTML = `
                <div class="folder-info">
                    <span class="folder-path" title="${folder.path}">${folder.path}</span>
                    <span class="folder-stats">${folder.decided_count}/${folder.photo_count} triaged</span>
                </div>
                <button class="remove-folder-btn" data-path="${folder.path}">&times;</button>
            `;
            foldersList.appendChild(item);
        });

        document.querySelectorAll('.remove-folder-btn').forEach(btn => {
            btn.addEventListener('click', () => confirmDeleteFolder(btn.dataset.path));
        });
    } catch (e) {
        console.error('Error loading folders:', e);
    }
}

async function openBrowser(target) {
    if (target) browserTarget = target;

    try {
        const homeDir = await invoke('get_home_dir');
        browserCurrentPath = homeDir;
        await loadBrowserContents(browserCurrentPath);
        folderBrowserModal.style.display = 'flex';
    } catch (error) {
        console.error('Error opening browser:', error);
        alert('Failed to open folder browser: ' + error);
    }
}

function closeBrowser() {
    folderBrowserModal.style.display = 'none';
    browserTarget = null;
}

async function loadBrowserContents(path) {
    try {
        const data = await invoke('browse', { path });

        if (data.error) {
            alert(data.message);
            return;
        }

        browserCurrentPath = data.current_path;
        document.getElementById('browserCurrentPath').textContent = data.current_path;

        const quickAccessList = document.getElementById('quickAccessList');
        quickAccessList.innerHTML = '';
        data.quick_access.forEach(item => {
            const btn = document.createElement('button');
            btn.className = 'quick-access-item';
            btn.textContent = item.name;
            btn.addEventListener('click', () => loadBrowserContents(item.path));
            quickAccessList.appendChild(btn);
        });

        const folderList = document.getElementById('browserFolderList');
        folderList.innerHTML = '';

        const dirs = data.items.filter(item => item.is_dir);

        if (dirs.length === 0) {
            folderList.innerHTML = '<div class="browser-empty">No subfolders</div>';
        } else {
            dirs.forEach(item => {
                const div = document.createElement('div');
                div.className = 'browser-folder-item';
                div.innerHTML = `<span class="folder-icon">&#128193;</span> ${item.name}`;
                div.addEventListener('dblclick', () => loadBrowserContents(item.path));
                div.addEventListener('click', () => {
                    document.querySelectorAll('.browser-folder-item').forEach(el => el.classList.remove('selected'));
                    div.classList.add('selected');
                });
                folderList.appendChild(div);
            });
        }
    } catch (e) {
        console.error('Error loading browser:', e);
        alert('Failed to load directory');
    }
}

async function navigateUp() {
    const parts = browserCurrentPath.split('/').filter(p => p);
    if (parts.length > 0) {
        parts.pop();
        const parent = '/' + parts.join('/');
        await loadBrowserContents(parent || '/');
    }
}

async function selectFolder() {
    const path = browserCurrentPath;

    // Handle setup screen selections
    if (browserTarget === 'setup_source') {
        if (!setupConfig.source_folders.includes(path)) {
            setupConfig.source_folders.push(path);
        }
        closeBrowser();
        updateSetupUI();
        return;
    }

    if (browserTarget === 'setup_accepted') {
        setupConfig.accepted_folder = path;
        closeBrowser();
        updateSetupUI();
        return;
    }

    if (browserTarget === 'setup_rejected') {
        setupConfig.rejected_folder = path;
        closeBrowser();
        updateSetupUI();
        return;
    }

    // Handle settings selections
    if (browserTarget === 'source') {
        try {
            await invoke('add_source_folder', { path });
            closeBrowser();
            await loadFolders();
            await loadCurrentImage();
        } catch (e) {
            console.error('Error adding folder:', e);
            alert('Failed to add folder: ' + e);
        }
    } else {
        try {
            await invoke('set_destination_folder', { folderType: browserTarget, path });
            closeBrowser();
            await loadFolders();
        } catch (e) {
            console.error('Error saving destination folder:', e);
            alert('Failed to update folder: ' + e);
        }
    }
}

function confirmDeleteFolder(path) {
    folderToDelete = path;
    confirmDeleteModal.style.display = 'flex';
}

async function deleteFolder(clearDecisions) {
    if (!folderToDelete) return;

    try {
        await invoke('remove_source_folder', { path: folderToDelete, clearDecisions });
        folderToDelete = null;
        confirmDeleteModal.style.display = 'none';
        await loadFolders();
        await loadCurrentImage();
    } catch (e) {
        console.error('Error deleting folder:', e);
    }
}

// ==================== PHOTO BROWSER ====================

let photoBrowserModal, photoBrowserGrid, browserTitle;
let browseAcceptedBtn, browseRejectedBtn, browserSortSelect;
let browserPrevBtn, browserNextBtn, browserPageInfo;

let browserStatus = 'accepted';
let browserSort = 'ranking';
let browserPage = 1;
let browserPerPage = 30;
let browserTotalPages = 1;

function initPhotoBrowser() {
    photoBrowserModal = document.getElementById('photoBrowserModal');
    photoBrowserGrid = document.getElementById('photoBrowserGrid');
    browserTitle = document.getElementById('browserTitle');
    browseAcceptedBtn = document.getElementById('browseAcceptedBtn');
    browseRejectedBtn = document.getElementById('browseRejectedBtn');
    browserSortSelect = document.getElementById('browserSortSelect');
    browserPrevBtn = document.getElementById('browserPrevBtn');
    browserNextBtn = document.getElementById('browserNextBtn');
    browserPageInfo = document.getElementById('browserPageInfo');

    // Browse button in header
    document.getElementById('browsePhotosBtn').addEventListener('click', openPhotoBrowser);

    // Close button
    document.getElementById('closePhotoBrowser').addEventListener('click', closePhotoBrowser);
    photoBrowserModal.addEventListener('click', (e) => {
        if (e.target === photoBrowserModal) closePhotoBrowser();
    });

    // Tab buttons
    browseAcceptedBtn.addEventListener('click', () => {
        browserStatus = 'accepted';
        browserPage = 1;
        updateBrowserTabs();
        loadBrowserPhotos();
    });

    browseRejectedBtn.addEventListener('click', () => {
        browserStatus = 'rejected';
        browserPage = 1;
        updateBrowserTabs();
        loadBrowserPhotos();
    });

    // Sort select
    browserSortSelect.addEventListener('change', () => {
        browserSort = browserSortSelect.value;
        browserPage = 1;
        loadBrowserPhotos();
    });

    // Pagination
    browserPrevBtn.addEventListener('click', () => {
        if (browserPage > 1) {
            browserPage--;
            loadBrowserPhotos();
        }
    });

    browserNextBtn.addEventListener('click', () => {
        if (browserPage < browserTotalPages) {
            browserPage++;
            loadBrowserPhotos();
        }
    });
}

function updateBrowserTabs() {
    browseAcceptedBtn.classList.toggle('active', browserStatus === 'accepted');
    browseRejectedBtn.classList.toggle('active', browserStatus === 'rejected');
    browserTitle.textContent = browserStatus === 'accepted' ? 'Accepted Photos' : 'Rejected Photos';
}

async function openPhotoBrowser() {
    browserPage = 1;
    browserStatus = 'accepted';
    updateBrowserTabs();
    photoBrowserModal.style.display = 'flex';
    await loadBrowserPhotos();
}

function closePhotoBrowser() {
    photoBrowserModal.style.display = 'none';
}

async function loadBrowserPhotos() {
    try {
        const data = await invoke('get_photos_by_status', {
            status: browserStatus,
            sort: browserSort,
            page: browserPage,
            perPage: browserPerPage
        });

        browserTotalPages = data.total_pages;

        // Update pagination
        browserPageInfo.textContent = `Page ${data.page} of ${data.total_pages} (${data.total} photos)`;
        browserPrevBtn.disabled = data.page <= 1;
        browserNextBtn.disabled = data.page >= data.total_pages;

        // Render photos
        photoBrowserGrid.innerHTML = '';

        if (data.photos.length === 0) {
            photoBrowserGrid.innerHTML = '<div class="no-photos">No photos found</div>';
            return;
        }

        data.photos.forEach((photo, idx) => {
            const rank = (data.page - 1) * data.per_page + idx + 1;
            const item = document.createElement('div');
            item.className = 'leaderboard-item';

            let scoreHtml = '';
            if (photo.score !== null && photo.score !== undefined) {
                scoreHtml = `
                    <div class="leaderboard-score">
                        <span class="score">${Math.round(photo.score)}</span>
                        <span class="mu-sigma">${Math.round(photo.mu)} ± ${Math.round(photo.sigma)}</span>
                    </div>
                `;
            } else {
                scoreHtml = `
                    <div class="leaderboard-score">
                        <span class="score" style="color: #666;">--</span>
                        <span class="mu-sigma">Not ranked</span>
                    </div>
                `;
            }

            item.innerHTML = `
                <span class="rank">#${rank}</span>
                <img src="${convertFileSrc(photo.file_path)}" alt="${photo.filename}" loading="lazy">
                ${scoreHtml}
                <div class="filename" title="${photo.filename}">${photo.filename}</div>
            `;

            // Click to open in lightbox
            item.querySelector('img').addEventListener('click', () => {
                openLightbox(convertFileSrc(photo.file_path));
            });

            photoBrowserGrid.appendChild(item);
        });
    } catch (e) {
        console.error('Error loading browser photos:', e);
        photoBrowserGrid.innerHTML = '<div class="no-photos">Error loading photos</div>';
    }
}

// Initialize on load
document.addEventListener('DOMContentLoaded', () => {
    init();
    initPhotoBrowser();
});
