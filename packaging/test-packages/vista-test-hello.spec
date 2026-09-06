Name:           vista-test-hello
Version:        1.0.0
Release:        1%{?dist}
Summary:        Vista test package - dummy hello tool
License:        MIT
BuildArch:      noarch

%description
Dummy test package for Vista resolver/DNF integration tests.
Provides /usr/bin/vista-hello. Used to verify Vista records
GitHub vs native vs Flathub sources correctly.

%install
mkdir -p %{buildroot}%{_bindir}
cat > %{buildroot}%{_bindir}/vista-hello <<'EOS'
#!/bin/bash
echo "vista-test-hello 1.0.0 - ok"
EOS
chmod 0755 %{buildroot}%{_bindir}/vista-hello

%files
%{_bindir}/vista-hello

%changelog
* Sun Sep 06 2026 Vista Contributors <vista@local> - 1.0.0-1
- Test package
