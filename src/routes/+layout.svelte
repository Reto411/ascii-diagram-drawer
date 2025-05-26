<script lang="ts">
    // ①  CSS first – this lets any of your later global.scss overrides win
    // import 'bootstrap/dist/css/bootstrap.min.css';
    // import 'bootstrap/dist/js/bootstrap.bundle.min.js';
</script>

<nav class="navbar">
    <ul class="navbar-nav">
        <li class="nav-item dropdown">
            <p class="nav-link">File</p>
            <ul class="dropdown-menu">
                <li><p class="dropdown-item">New</p></li>
                <li><p class="dropdown-item">Save as ...</p></li>
            </ul>
        </li>
        <li class="nav-item dropdown">
            <p class="nav-link">Settings</p>
            <ul class="dropdown-menu">
                <li><p class="dropdown-item" >Preferences</p></li>
            </ul>
        </li>
    </ul>
</nav>
<slot  />
<footer class="footer">
    <div class="container-fluid bg-light">
        Ascii Diagrams Drawer
    </div>
</footer>
