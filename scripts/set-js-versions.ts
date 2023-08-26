// Called from .github/workflows/check-ts.yml

import fs from 'fs'
import path from 'path'

interface IncorrectDependency {
  packageFile: string
  packageName: string
  dependentPackage: string
  expectedVersion: string
  actualVersion: string
}

function getIncorrectDependencies(packagesMap: Record<string, PackageData>): IncorrectDependency[] {
  const incorrectDependencies: IncorrectDependency[] = []

  // Go over the dependencies of each package
  Object.entries(packagesMap).forEach(([, packageData]) => {
    const { dependencies } = packageData
    Object.entries(dependencies || []).forEach(([dependencyName, version]) => {
      if (packagesMap[dependencyName]) {
        const actualVersion = version
        const expectedVersion = packagesMap[dependencyName].version
        if (actualVersion !== expectedVersion) {
          incorrectDependencies.push({
            packageFile: packageData.packageFile,
            packageName: packageData.name,
            dependentPackage: dependencyName,
            expectedVersion,
            actualVersion,
          })
        }
      }
    })
  })

  return incorrectDependencies
}

interface PackageData {
  packageFile: string
  name: string
  version: string
  dependencies?: Record<string, string>
  private?: boolean
}

function main() {
  const args = process.argv.slice(2)

  let versionToSet: string | undefined = undefined
  if (args.length > 1) {
    console.error('Too many arguments')
    process.exit(1)
  } else if (args.length === 1) {
    versionToSet = args[0]

    if (!versionToSet.match(/^\d+\.\d+\.\d+$/)) {
      console.error('Invalid version format')
      process.exit(1)
    }
  } else {
    console.info('No argument passed; only checking.')
  }

  const rootPackageJsonPath = 'package.json'
  const rootPackageJson = JSON.parse(fs.readFileSync(rootPackageJsonPath, 'utf-8'))

  if (!rootPackageJson.workspaces || !Array.isArray(rootPackageJson.workspaces)) {
    console.error('No workspaces found in root package.json')
    process.exit(1)
  }

  const packagesMap: Record<string, PackageData> = {}

  // For each package in the workspace, read the package.json and collect the info
  rootPackageJson.workspaces.forEach((workspace: string) => {
    const packageJsonPath = path.join(workspace, 'package.json')
    const packageData = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'))

    packagesMap[packageData.name] = {
      packageFile: packageJsonPath,
      name: packageData.name,
      version: packageData.version,
      dependencies: packageData.dependencies || {},
      private: packageData.private,
    }
  })

  if (versionToSet !== undefined) {
    const newVersion = versionToSet
    // Set the version in all packages
    Object.entries(packagesMap).forEach(([, packageData]) => {
      const packageJson = JSON.parse(fs.readFileSync(packageData.packageFile, 'utf-8'))

      if (!packageData.private) {
        packageJson.version = newVersion
      }

      // Update the dependencies
      Object.entries(packageJson.dependencies || []).forEach(([dependencyName, _]) => {
        if (packagesMap[dependencyName]) {
          packageJson.dependencies![dependencyName] = newVersion
        }
      })

      fs.writeFileSync(packageData.packageFile, JSON.stringify(packageJson, null, 4))

      console.log(`Updated ${packageData.packageFile} to version ${newVersion}`)
    })

    console.log(`Version set to ${versionToSet}`)
  } else {
    // Check existing dependencies.

    const incorrectDependencies = getIncorrectDependencies(packagesMap)

    // Print the list of incorrect dependencies
    if (incorrectDependencies.length > 0) {
      console.error('Incorrect dependencies found:')
      incorrectDependencies.forEach((dep) => {
        console.error(`Package: ${dep.packageName} (${dep.packageFile})`)
        console.error(`Depends on: ${dep.dependentPackage}`)
        console.error(`Expected version: ${dep.expectedVersion}`)
        console.error(`Actual version: ${dep.actualVersion}`)
        console.error('---------------------')
      })

      // Exit with a value 1
      process.exit(1)
    } else {
      console.log('Packages checked:')
      Object.entries(packagesMap).forEach(([name, packageData]) => {
        console.log(`    ${packageData.name}@${packageData.version} (${packageData.packageFile})`)
      })
      console.log('All dependencies are correct.')
    }
  }
}

main()
