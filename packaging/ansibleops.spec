%global debug_package %{nil}

%global ansibleops_version 1.2.0
%global ansibleops_release 1

Name:           ansibleops
Version:        %{ansibleops_version}
Release:        %{ansibleops_release}%{?dist}
Summary:        AnsibleOps — desktop control center for Ansible and Terraform

License:        MIT
URL:            https://github.com/flytimopheev-sketch/ansibleops
Source0:        %{name}-%{ansibleops_version}.tar.gz

BuildArch:      x86_64
BuildRequires:  cargo >= 1.70
BuildRequires:  rust
BuildRequires:  gcc
BuildRequires:  make
BuildRequires:  pkgconfig
BuildRequires:  libX11-devel
BuildRequires:  libxcb-devel
BuildRequires:  mesa-libGL-devel
BuildRequires:  oniguruma-devel
BuildRequires:  findutils

Requires:       libX11
Requires:       libxcb
Requires:       mesa-libGL
Requires:       fontconfig
Requires:       glibc >= 2.17
Requires:       libgcc

%description
AnsibleOps is a desktop control center for Ansible and Terraform.
It provides playbook execution with live logs, cancellation and secret
masking, a visual playbook constructor, YAML editor, task dependency graph,
inventory and ad-hoc management, roles catalog, diff scanning of
--check --diff output, GitOps, compliance checks, scheduling, reports,
a local RPM repository, Vault, Zabbix notifications and a read-only Web API.

%prep
%setup -q -n %{name}-%{ansibleops_version}

%build
export CARGO_TARGET_DIR=%{_builddir}/%{name}-%{ansibleops_version}/target
cargo build --release

%install
rm -rf %{buildroot}
install -D -m 0755 target/release/ansibleops %{buildroot}%{_bindir}/ansibleops

install -D -m 0644 packaging/ansibleops.desktop %{buildroot}%{_datadir}/applications/ansibleops.desktop
install -D -m 0644 logo.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/ansibleops.svg
install -D -m 0644 LICENSE %{buildroot}%{_datadir}/licenses/%{name}/LICENSE
install -D -m 0644 README.md %{buildroot}%{_datadir}/doc/%{name}/README.md

install -D -m 0644 packaging/ansibleops.service %{buildroot}%{_unitdir}/ansibleops.service

%check
# %{__ldd} на EL7 падает с "fg: no job control" — вызываем ldd напрямую.
ldd %{buildroot}%{_bindir}/ansibleops

%files
%license %{_datadir}/licenses/%{name}/LICENSE
%doc %{_datadir}/doc/%{name}/README.md
%{_bindir}/ansibleops
%{_datadir}/applications/ansibleops.desktop
%{_datadir}/icons/hicolor/scalable/apps/ansibleops.svg
%{_unitdir}/ansibleops.service

%changelog
* Wed Oct 21 2026 flytimopheev-sketch <flytimopheev-sketch@users.noreply.github.com> - 1.2.0-1
- Initial RPM release of AnsibleOps built for RED OS 7 / EL7.
