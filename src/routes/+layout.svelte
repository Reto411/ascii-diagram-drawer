<script lang="ts">
    // ①  CSS first – this lets any of your later global.scss overrides win
    // import 'bootstrap/dist/css/bootstrap.min.css';
    // import 'bootstrap/dist/js/bootstrap.bundle.min.js';
</script>

<nav class="navbar">
    <ul class="navbar-nav">
        <li class="nav-item dropdown">
            <a href="#" class="nav-link">File</a>
            <ul class="dropdown-menu">
                <li><a class="dropdown-item" href="#">New</a></li>
                <li><a class="dropdown-item" href="#">Save as ...</a></li>
            </ul>
        </li>
        <li class="nav-item dropdown">
            <a href="#" class="nav-link">Settings</a>
            <ul class="dropdown-menu">
                <li><a class="dropdown-item" href="#">Preferences</a></li>
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
