Name:           vista
Version:        0.1.0
Release:        1%{?dist}
Summary:        Vista — GitHub-backed universal Linux package manager
License:        MIT
URL:            https://github.com/whyfle/vista
Source0:        %{name}-%{version}.tar.gz
BuildRequires:  cargo
BuildRequires:  rust
Requires:       flatpak
Requires:       dnf
BuildArch:      x86_64

%description
Vista tells you what you want, Vista figures out how your Linux system
should install it. GitHub Releases as primary backend, native RPM/DEB
resolution with scoring, Flatpak/Flathub fallback.

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release --locked

%install
install -Dm0755 target/release/vista %{buildroot}%{_bindir}/vista
install -Dm0644 packaging/repo/vista.repo %{buildroot}%{_sysconfdir}/yum.repos.d/vista.repo
install -Dm0644 packaging/flatpak/io.github.whyfle.Vista.metainfo.xml %{buildroot}%{_datadir}/metainfo/io.github.whyfle.Vista.metainfo.xml
# default config example
install -Dm0644 packaging/config-example.toml %{buildroot}%{_datadir}/doc/vista/config-example.toml

%files
%{_bindir}/vista
%{_sysconfdir}/yum.repos.d/vista.repo
%{_datadir}/metainfo/io.github.whyfle.Vista.metainfo.xml
%{_datadir}/doc/vista/config-example.toml
%doc README.md LICENSE

%changelog
* Sun Sep 06 2026 Vista Contributors <vista@local> - 0.1.0-1
- Initial package
