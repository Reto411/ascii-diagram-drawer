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

<style>
    /* Navbar dropdowns (pure CSS) */
    .navbar-nav {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
    }

    .nav-item {
        position: relative;
    }

    .nav-link {
        display: block;
        padding: 1rem;
        color: #333;
        text-decoration: none;
        cursor: pointer;
    }

    .nav-link:hover,
    .nav-link:focus {
        background: #e2e6ea;
    }

    .dropdown-menu {
        display: none;
        position: absolute;
        top: 100%;
        left: 0;
        min-width: 160px;
        background: #fff;
        border: 1px solid #ccc;
        z-index: 1000;
        list-style: none;
        margin: 0;
        padding: 0;
    }

    .nav-item:hover .dropdown-menu,
    .nav-item:focus-within .dropdown-menu {
        display: block;
    }

    .dropdown-item {
        padding: 0.5rem 1rem;
        color: #333;
        text-decoration: none;
        display: block;
        white-space: nowrap;
    }

    .dropdown-item:hover,
    .dropdown-item:focus {
        background: #f8f9fa;
    }
</style>