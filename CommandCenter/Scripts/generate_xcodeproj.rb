#!/usr/bin/env ruby

require 'fileutils'
require 'pathname'
require 'xcodeproj'

ROOT = Pathname(__dir__).parent.expand_path
PROJECT_PATH = ROOT.join('CommandCenter.xcodeproj')
CONFIG_PATH = ROOT.join('Config')
SOURCES_PATH = ROOT.join('Sources')
TESTS_PATH = ROOT.join('Tests')

FileUtils.rm_rf(PROJECT_PATH)

project = Xcodeproj::Project.new(PROJECT_PATH.to_s)
app_target = project.new_target(:application, 'CommandCenter', :osx, '26.0')
test_target = project.new_target(:unit_test_bundle, 'CommandCenterTests', :osx, '26.0')

project.build_configurations.each do |config|
    config.build_settings['MACOSX_DEPLOYMENT_TARGET'] = '26.0'
    config.build_settings['SWIFT_VERSION'] = '6.0'
end

app_target.build_configurations.each do |config|
    config.build_settings['PRODUCT_BUNDLE_IDENTIFIER'] = 'com.convergio.CommandCenter'
    config.build_settings['PRODUCT_NAME'] = '$(TARGET_NAME)'
    config.build_settings['SWIFT_VERSION'] = '6.0'
    config.build_settings['MACOSX_DEPLOYMENT_TARGET'] = '26.0'
    config.build_settings['GENERATE_INFOPLIST_FILE'] = 'NO'
    config.build_settings['INFOPLIST_FILE'] = 'Config/Info.plist'
    config.build_settings['CODE_SIGN_ENTITLEMENTS'] = 'Config/CommandCenter.entitlements'
    config.build_settings['ENABLE_HARDENED_RUNTIME'] = 'YES'
    config.build_settings['CODE_SIGN_STYLE'] = 'Automatic'
    config.build_settings['CODE_SIGNING_ALLOWED'] = 'NO'
    config.build_settings['CODE_SIGNING_REQUIRED'] = 'NO'
    config.build_settings['LD_RUNPATH_SEARCH_PATHS'] = '$(inherited) @executable_path/../Frameworks'
    config.build_settings['SUPPORTED_PLATFORMS'] = 'macosx'
end

test_target.build_configurations.each do |config|
    config.build_settings['PRODUCT_BUNDLE_IDENTIFIER'] = 'com.convergio.CommandCenterTests'
    config.build_settings['PRODUCT_NAME'] = '$(TARGET_NAME)'
    config.build_settings['SWIFT_VERSION'] = '6.0'
    config.build_settings['MACOSX_DEPLOYMENT_TARGET'] = '26.0'
    config.build_settings['GENERATE_INFOPLIST_FILE'] = 'YES'
    config.build_settings['CODE_SIGN_STYLE'] = 'Automatic'
    config.build_settings['CODE_SIGNING_ALLOWED'] = 'NO'
    config.build_settings['CODE_SIGNING_REQUIRED'] = 'NO'
    config.build_settings['SUPPORTED_PLATFORMS'] = 'macosx'
end

test_target.add_system_framework('XCTest')

def ensure_group(parent, relative_path)
    current = parent
    relative_path.each_filename do |component|
        next if component.to_s.empty? || component.to_s == '.'

        current = current.groups.find { |group| group.path == component.to_s } ||
            current.new_group(component.to_s, component.to_s)
    end
    current
end

source_refs = Dir.glob(SOURCES_PATH.join('**/*.swift')).sort.map do |source|
    relative = Pathname(source).relative_path_from(ROOT)
    group = ensure_group(project.main_group, relative.dirname)
    group.new_file(relative.basename.to_s)
end

app_target.add_file_references(source_refs)

test_refs = Dir.glob(TESTS_PATH.join('**/*.swift')).sort.map do |source|
    relative = Pathname(source).relative_path_from(ROOT)
    group = ensure_group(project.main_group, relative.dirname)
    group.new_file(relative.basename.to_s)
end

test_target.add_file_references(test_refs) unless test_refs.empty?

config_group = ensure_group(project.main_group, Pathname('Config'))
%w[Info.plist CommandCenter.entitlements].each do |name|
    config_group.new_file(name)
end

project.save

scheme = Xcodeproj::XCScheme.new
scheme.configure_with_targets(app_target, test_target, launch_target: true)
scheme.save_as(PROJECT_PATH.to_s, 'CommandCenter', true)
